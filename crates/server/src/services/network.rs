use std::time::Duration;

use tracing::info;

use super::{command_runner::run_command_with_timeout, map_run_output, SystemExecResult};

pub(super) async fn run_wifi_provision(ssid: &str, pwd: Option<&str>) -> SystemExecResult {
    if ssid.is_empty() {
        return SystemExecResult::error(protocol::codes::CODE_BAD_REQUEST, "SSID cannot be empty");
    }

    info!("Starting Wi-Fi provisioning to SSID: {}", ssid);

    let mut cmd = vec!["nmcli", "device", "wifi", "connect", ssid];
    if let Some(pwd) = pwd {
        cmd.push("password");
        cmd.push(pwd);
    }

    let result = map_run_output(run_command_with_timeout(cmd, 30.0).await, 30.0);
    if result.ok {
        let ip_result = map_run_output(
            run_command_with_timeout(vec!["hostname", "-I"], 2.0).await,
            2.0,
        );
        let ip = if ip_result.ok {
            Some(ip_result.text.trim().to_string())
        } else {
            None
        };
        finalize_wifi_provision(ssid, result, ip)
    } else {
        finalize_wifi_provision(ssid, result, None)
    }
}

pub(super) async fn run_wifi_scan(ifname: Option<&str>) -> SystemExecResult {
    let mut rescan_cmd = vec!["nmcli", "device", "wifi", "rescan"];
    if let Some(ifname) = ifname {
        rescan_cmd.extend_from_slice(&["ifname", ifname]);
    }
    let rescan = map_run_output(run_command_with_timeout(rescan_cmd, 6.0).await, 6.0);
    if !rescan.ok {
        return rescan;
    }

    tokio::time::sleep(Duration::from_secs(5)).await;

    let mut list_cmd = vec![
        "nmcli",
        "-t",
        "-f",
        "IN-USE,BSSID,SSID,CHAN,RATE,SIGNAL,BARS,SECURITY",
        "device",
        "wifi",
        "list",
    ];
    if let Some(ifname) = ifname {
        list_cmd.extend_from_slice(&["ifname", ifname]);
    }

    let listed = map_run_output(run_command_with_timeout(list_cmd, 8.0).await, 8.0);
    if !listed.ok {
        return listed;
    }

    let networks = parse_nmcli_wifi_list(&listed.text);
    let data = protocol::responses::WifiScanResponseData {
        ifname: ifname.map(|value| value.to_string()),
        count: networks.len() as u64,
        networks,
    };

    SystemExecResult::ok(
        "wifi scan complete",
        Some(protocol::responses::to_map(&data).expect("wifi scan response serializes")),
    )
}

pub(super) fn parse_nmcli_wifi_list(list_output: &str) -> Vec<protocol::responses::WifiNetwork> {
    list_output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let parts = split_nmcli_fields(line);
            if parts.len() < 8 {
                return None;
            }

            let ssid = parts[2].as_str();
            if ssid.is_empty() {
                return None;
            }

            let signal = match parts[5].parse::<i32>() {
                Ok(signal) => signal,
                Err(_) => return None,
            };
            Some(protocol::responses::WifiNetwork {
                ssid: ssid.to_string(),
                channel: parts[3].clone(),
                signal,
            })
        })
        .collect()
}

fn split_nmcli_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaping = false;

    for ch in line.chars() {
        if escaping {
            current.push(ch);
            escaping = false;
            continue;
        }

        match ch {
            '\\' => escaping = true,
            ':' => {
                fields.push(current);
                current = String::new();
            }
            _ => current.push(ch),
        }
    }

    fields.push(current);
    fields
}

pub(super) fn finalize_wifi_provision(
    ssid: &str,
    result: SystemExecResult,
    ip: Option<String>,
) -> SystemExecResult {
    if result.ok {
        let data = protocol::responses::ProvisionResponseData {
            status: protocol::responses::ProvisionState::Connected,
            ssid: ssid.to_string(),
            ip,
        };

        return SystemExecResult::with_code(
            true,
            protocol::codes::CODE_PROVISION_SUCCESS,
            format!("Provisioned Wi-Fi for {ssid}"),
            Some(protocol::responses::to_map(&data).expect("provision response serializes")),
        );
    }

    let data = protocol::responses::ProvisionResponseData {
        status: protocol::responses::ProvisionState::Failed,
        ssid: ssid.to_string(),
        ip: None,
    };

    let code = if result.code == protocol::codes::CODE_TIMEOUT {
        protocol::codes::CODE_TIMEOUT
    } else if result.code == protocol::codes::CODE_BAD_REQUEST {
        protocol::codes::CODE_BAD_REQUEST
    } else {
        protocol::codes::CODE_PROVISION_FAIL
    };

    SystemExecResult::with_code(
        false,
        code,
        format!("Provisioning failed: {}", result.text),
        Some(protocol::responses::to_map(&data).expect("provision response serializes")),
    )
}
