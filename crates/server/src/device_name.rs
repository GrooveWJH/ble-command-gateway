use chrono::Local;
use std::fs;

pub fn generate_device_name(base_prefix: &str) -> String {
    let boot_hh_mm = Local::now().format("%H-%M").to_string();
    let machine_id = fs::read_to_string("/etc/machine-id").ok();
    build_device_name_from_parts(base_prefix, &boot_hh_mm, machine_id.as_deref())
}

pub fn build_device_name_from_parts(
    base_prefix: &str,
    boot_hh_mm: &str,
    machine_id: Option<&str>,
) -> String {
    let short_id = short_machine_id_from_text(machine_id.unwrap_or_default())
        .unwrap_or_else(|| "UNK".to_string());
    format!("{base_prefix}-{boot_hh_mm}-{short_id}")
}

pub fn short_machine_id_from_text(machine_id: &str) -> Option<String> {
    let normalized: String = machine_id
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect();

    if normalized.is_empty() {
        return None;
    }

    let mut hash = 0x811C9DC5u32;
    for byte in normalized.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }

    Some(format!("{:04X}", hash & 0xFFFF))
}

#[cfg(test)]
mod tests {
    use super::{build_device_name_from_parts, short_machine_id_from_text};

    #[test]
    fn device_name_uses_time_and_short_id() {
        let name = build_device_name_from_parts("Yundrone_UAV", "15-19", Some("abcdef1234567890"));

        assert_eq!(name, "Yundrone_UAV-15-19-D8AB");
    }

    #[test]
    fn device_name_falls_back_to_unknown_suffix() {
        let name = build_device_name_from_parts("Yundrone_UAV", "15-19", None);

        assert_eq!(name, "Yundrone_UAV-15-19-UNK");
    }

    #[test]
    fn short_machine_id_is_stable_uppercase_hex() {
        let short_id = short_machine_id_from_text("abcdef1234567890").unwrap();

        assert_eq!(short_id, "D8AB");
        assert_eq!(short_id.len(), 4);
        assert!(short_id
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_lowercase()));
    }
}
