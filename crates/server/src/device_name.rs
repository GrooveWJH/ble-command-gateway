use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

pub const DEFAULT_DEVICE_NAME_PATH: &str = "/var/lib/yundrone/ble-device-name";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceNameSource {
    File,
    Generated { reason: String },
}

impl DeviceNameSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Generated { .. } => "generated",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDeviceName {
    pub name: String,
    pub source: DeviceNameSource,
}

pub fn generate_device_name(base_prefix: &str) -> String {
    let machine_id = fs::read_to_string("/etc/machine-id").ok();
    build_device_name_from_parts(base_prefix, machine_id.as_deref())
}

pub fn resolve_persisted_device_name(
    base_prefix: &str,
    path: &Path,
) -> std::io::Result<ResolvedDeviceName> {
    let machine_id = fs::read_to_string("/etc/machine-id").ok();
    resolve_persisted_device_name_with_machine_id(base_prefix, path, machine_id.as_deref())
}

fn resolve_persisted_device_name_with_machine_id(
    base_prefix: &str,
    path: &Path,
    machine_id: Option<&str>,
) -> std::io::Result<ResolvedDeviceName> {
    match fs::read_to_string(path) {
        Ok(content) => {
            if let Some(name) = parse_persisted_name(base_prefix, &content) {
                return Ok(ResolvedDeviceName {
                    name,
                    source: DeviceNameSource::File,
                });
            }
            persist_generated_name(base_prefix, path, machine_id, "invalid")
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            persist_generated_name(base_prefix, path, machine_id, "missing")
        }
        Err(err) => Err(err),
    }
}

fn parse_persisted_name(base_prefix: &str, content: &str) -> Option<String> {
    let line = content
        .strip_suffix("\r\n")
        .or_else(|| content.strip_suffix('\n'))
        .or_else(|| content.strip_suffix('\r'))
        .unwrap_or(content);

    if line.is_empty() || line != line.trim() || line.contains(['\r', '\n']) {
        return None;
    }
    is_stable_device_name(base_prefix, line).then(|| line.to_string())
}

pub fn is_stable_device_name(base_prefix: &str, value: &str) -> bool {
    let Some(suffix) = value
        .strip_prefix(base_prefix)
        .and_then(|rest| rest.strip_prefix('-'))
    else {
        return false;
    };
    suffix.len() == 6
        && suffix
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch.is_ascii_lowercase())
}

fn persist_generated_name(
    base_prefix: &str,
    path: &Path,
    machine_id: Option<&str>,
    reason: &str,
) -> std::io::Result<ResolvedDeviceName> {
    let name = build_device_name_from_parts(base_prefix, machine_id);
    write_device_name_atomic(path, &name)?;
    Ok(ResolvedDeviceName {
        name,
        source: DeviceNameSource::Generated {
            reason: reason.to_string(),
        },
    })
}

fn write_device_name_atomic(path: &Path, name: &str) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("device name path has no parent: {}", path.display()),
        )
    })?;
    fs::create_dir_all(parent)?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, format!("{name}\n"))?;
    set_device_name_permissions(parent, &tmp_path)?;
    fs::rename(tmp_path, path)?;
    set_device_name_permissions(parent, path)?;
    Ok(())
}

#[cfg(unix)]
fn set_device_name_permissions(dir: &Path, file: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(dir, fs::Permissions::from_mode(0o755))?;
    fs::set_permissions(file, fs::Permissions::from_mode(0o644))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_device_name_permissions(_dir: &Path, _file: &Path) -> std::io::Result<()> {
    Ok(())
}

pub fn build_device_name_from_parts(base_prefix: &str, machine_id: Option<&str>) -> String {
    let suffix =
        stable_base36_suffix_from_text(machine_id.unwrap_or_default()).unwrap_or_else(|| {
            stable_base36_suffix_from_text(base_prefix).expect("prefix is non-empty")
        });
    format!("{base_prefix}-{suffix}")
}

pub fn stable_base36_suffix_from_text(machine_id: &str) -> Option<String> {
    let normalized: String = machine_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect();

    if normalized.is_empty() {
        return None;
    }

    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    for byte in normalized.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }

    Some(to_fixed_base36(hash % 36_u64.pow(6), 6))
}

fn to_fixed_base36(mut value: u64, width: usize) -> String {
    const ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut output = vec![b'0'; width];
    for slot in output.iter_mut().rev() {
        *slot = ALPHABET[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8(output).expect("base36 alphabet is valid utf-8")
}

#[cfg(test)]
mod tests {
    use super::{
        build_device_name_from_parts, resolve_persisted_device_name_with_machine_id,
        stable_base36_suffix_from_text, DeviceNameSource,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "yundrone-device-name-{name}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn file(&self) -> PathBuf {
            self.path.join("ble-device-name")
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn device_name_uses_stable_base36_suffix() {
        let name = build_device_name_from_parts("yundrone", Some("abcdef1234567890"));

        assert_eq!(name, "yundrone-ytcwln");
    }

    #[test]
    fn device_name_falls_back_to_unknown_suffix() {
        let name = build_device_name_from_parts("yundrone", None);

        assert_eq!(name, "yundrone-xsc8kb");
    }

    #[test]
    fn stable_suffix_is_lowercase_six_base36_chars() {
        let short_id = stable_base36_suffix_from_text("abcdef1234567890").unwrap();

        assert_eq!(short_id, "ytcwln");
        assert_eq!(short_id.len(), 6);
        assert!(short_id
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch.is_ascii_lowercase()));
    }

    #[test]
    fn persisted_name_uses_valid_existing_file() {
        let dir = TestDir::new("valid");
        fs::write(dir.file(), "yundrone-bw0uwj\n").unwrap();

        let resolved =
            resolve_persisted_device_name_with_machine_id("yundrone", &dir.file(), None).unwrap();

        assert_eq!(resolved.name, "yundrone-bw0uwj");
        assert_eq!(resolved.source, DeviceNameSource::File);
    }

    #[test]
    fn persisted_name_creates_missing_file() {
        let dir = TestDir::new("missing");

        let resolved = resolve_persisted_device_name_with_machine_id(
            "yundrone",
            &dir.file(),
            Some("abcdef1234567890"),
        )
        .unwrap();

        assert_eq!(resolved.name, "yundrone-ytcwln");
        assert_eq!(
            resolved.source,
            DeviceNameSource::Generated {
                reason: "missing".to_string()
            }
        );
        assert_eq!(fs::read_to_string(dir.file()).unwrap(), "yundrone-ytcwln\n");
    }

    #[test]
    fn persisted_name_replaces_invalid_content() {
        for (name, content) in [
            ("empty", ""),
            ("spaces", " yundrone-bw0uwj\n"),
            ("extra-blank-line", "yundrone-bw0uwj\n\n"),
            ("multiline", "yundrone-bw0uwj\nextra\n"),
            ("internal-space", "yundrone-bw 0uwj\n"),
            ("legacy", "yundrone-07-44-5433\n"),
            ("uppercase", "yundrone-BW0UWJ\n"),
            ("wrong-prefix", "custom-bw0uwj\n"),
        ] {
            let dir = TestDir::new(name);
            fs::write(dir.file(), content).unwrap();

            let resolved = resolve_persisted_device_name_with_machine_id(
                "yundrone",
                &dir.file(),
                Some("abcdef1234567890"),
            )
            .unwrap();

            assert_eq!(resolved.name, "yundrone-ytcwln");
            assert!(matches!(
                resolved.source,
                DeviceNameSource::Generated { .. }
            ));
            assert_eq!(fs::read_to_string(dir.file()).unwrap(), "yundrone-ytcwln\n");
        }
    }

    #[cfg(unix)]
    #[test]
    fn persisted_name_sets_expected_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestDir::new("permissions");

        resolve_persisted_device_name_with_machine_id(
            "yundrone",
            &dir.file(),
            Some("abcdef1234567890"),
        )
        .unwrap();

        assert_eq!(
            fs::metadata(&dir.path).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(dir.file()).unwrap().permissions().mode() & 0o777,
            0o644
        );
    }
}
