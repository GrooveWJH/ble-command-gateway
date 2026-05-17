#[cfg(target_os = "linux")]
use tokio::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BluetoothdEnvironment {
    pub has_experimental: bool,
    pub command_line: Option<String>,
}

#[cfg(target_os = "linux")]
pub async fn inspect_bluetoothd_environment() -> BluetoothdEnvironment {
    let output = Command::new("ps")
        .args(["-eo", "args="])
        .output()
        .await;

    let command_line = output
        .ok()
        .filter(|value| value.status.success())
        .and_then(|value| {
            String::from_utf8(value.stdout)
                .ok()
                .and_then(|text| find_bluetoothd_command_line(&text))
        });

    BluetoothdEnvironment {
        has_experimental: command_line
            .as_deref()
            .is_some_and(|line| line.contains("--experimental")),
        command_line,
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn inspect_bluetoothd_environment() -> BluetoothdEnvironment {
    BluetoothdEnvironment {
        has_experimental: false,
        command_line: None,
    }
}

#[cfg(any(target_os = "linux", test))]
fn find_bluetoothd_command_line(process_list: &str) -> Option<String> {
    process_list
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && line.contains("bluetoothd")
                && !line.contains("grep bluetoothd")
                && !line.contains("rg bluetoothd")
        })
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::find_bluetoothd_command_line;

    #[test]
    fn extracts_running_bluetoothd_command_line() {
        let process_list = "\
/sbin/init\n\
/usr/lib/bluetooth/bluetoothd -d --experimental --noplugin=audio\n";

        let line = find_bluetoothd_command_line(process_list);

        assert_eq!(
            line.as_deref(),
            Some("/usr/lib/bluetooth/bluetoothd -d --experimental --noplugin=audio")
        );
    }

    #[test]
    fn ignores_non_matching_lines() {
        let process_list = "\
/usr/bin/grep bluetoothd\n\
/usr/bin/python3 worker.py\n";

        assert!(find_bluetoothd_command_line(process_list).is_none());
    }
}
