use super::{run_system_command, SystemExecResult};
use protocol::responses::{StatusInterfaceIpv4, StatusInterfaceKind};
use std::collections::HashMap;

pub(super) fn run_help() -> SystemExecResult {
    let data = protocol::responses::HelpResponseData {
        commands: vec![
            protocol::commands::CMD_HELP.to_string(),
            protocol::commands::CMD_PING.to_string(),
            protocol::commands::CMD_STATUS.to_string(),
            protocol::commands::CMD_PROVISION.to_string(),
            protocol::commands::CMD_SHUTDOWN.to_string(),
            protocol::commands::CMD_SYS_WHOAMI.to_string(),
            protocol::commands::CMD_NET_IFCONFIG.to_string(),
            protocol::commands::CMD_WIFI_SCAN.to_string(),
        ],
    };

    SystemExecResult::ok(
        "supported commands listed",
        Some(protocol::responses::to_map(&data).expect("help response serializes")),
    )
}

pub(super) fn run_ping() -> SystemExecResult {
    let data = protocol::responses::PingResponseData { pong: true };
    SystemExecResult::ok(
        "pong",
        Some(protocol::responses::to_map(&data).expect("ping response serializes")),
    )
}

pub(super) async fn run_status(timeout_sec: f64) -> SystemExecResult {
    let hostname = run_system_command(vec!["hostname"], timeout_sec).await;
    if !hostname.ok {
        return hostname;
    }

    let system = run_system_command(vec!["uname", "-srm"], timeout_sec).await;
    if !system.ok {
        return system;
    }

    let user = run_effective_user(timeout_sec).await;
    if !user.ok {
        return user;
    }

    let network = current_active_network(timeout_sec).await;
    let interfaces = current_status_interfaces(timeout_sec).await;
    let ip = preferred_ipv4(&interfaces);

    let data = protocol::responses::StatusResponseData {
        hostname: hostname.text.clone(),
        system: system.text.clone(),
        user: user.text.clone(),
        network,
        ip,
        interfaces,
    };

    SystemExecResult::ok(
        "status collected",
        Some(protocol::responses::to_map(&data).expect("status response serializes")),
    )
}

pub(super) async fn run_effective_user(timeout_sec: f64) -> SystemExecResult {
    if let Ok(user) = std::env::var("BLE_GATEWAY_USER") {
        let user = user.trim();
        if !user.is_empty() && user != "root" {
            let data = protocol::responses::WhoAmIResponseData {
                user: user.to_string(),
            };
            return SystemExecResult::ok(
                user,
                Some(protocol::responses::to_map(&data).expect("whoami response serializes")),
            );
        }
    }

    if let Ok(user) = std::env::var("SUDO_USER") {
        let user = user.trim();
        if !user.is_empty() && user != "root" {
            let data = protocol::responses::WhoAmIResponseData {
                user: user.to_string(),
            };
            return SystemExecResult::ok(
                user,
                Some(protocol::responses::to_map(&data).expect("whoami response serializes")),
            );
        }
    }

    let who = run_system_command(vec!["who"], timeout_sec).await;
    if who.ok {
        if let Some(user) = parse_preferred_login_user(&who.text) {
            let data = protocol::responses::WhoAmIResponseData { user: user.clone() };
            return SystemExecResult::ok(
                user,
                Some(protocol::responses::to_map(&data).expect("whoami response serializes")),
            );
        }
    }

    let result = run_system_command(vec!["whoami"], timeout_sec).await;
    if result.ok {
        let data = protocol::responses::WhoAmIResponseData {
            user: result.text.clone(),
        };
        SystemExecResult::ok(
            result.text,
            Some(protocol::responses::to_map(&data).expect("whoami response serializes")),
        )
    } else {
        result
    }
}

async fn current_active_network(timeout_sec: f64) -> Option<String> {
    let result = run_system_command(
        vec![
            "nmcli",
            "-t",
            "-f",
            "NAME,DEVICE",
            "connection",
            "show",
            "--active",
        ],
        timeout_sec,
    )
    .await;
    if !result.ok {
        return None;
    }
    parse_active_network_name(&result.text)
}

async fn current_status_interfaces(timeout_sec: f64) -> Vec<StatusInterfaceIpv4> {
    let ip_result = run_system_command(
        vec!["ip", "-4", "-o", "addr", "show", "scope", "global"],
        timeout_sec,
    )
    .await;
    if !ip_result.ok {
        return vec![];
    }

    let device_types_result = run_system_command(
        vec!["nmcli", "-t", "-f", "DEVICE,TYPE", "device", "status"],
        timeout_sec,
    )
    .await;

    let raw_interfaces = parse_ipv4_interfaces(&ip_result.text);
    let device_types = if device_types_result.ok {
        parse_device_types(&device_types_result.text)
    } else {
        HashMap::new()
    };

    build_status_interfaces(raw_interfaces, device_types)
}

pub(super) fn parse_active_network_name(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let (name, device) = if let Some((name, device)) = line.split_once(':') {
            (name.trim().to_string(), device.trim().to_string())
        } else {
            let mut parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 2 {
                return None;
            }
            let device = parts.pop()?.trim().to_string();
            let name = parts.join(" ");
            (name.trim().to_string(), device)
        };
        if name.is_empty()
            || device.is_empty()
            || device == "lo"
            || (name == "NAME" && device == "DEVICE")
        {
            None
        } else {
            Some(name.replace("\\:", ":"))
        }
    })
}

pub(super) fn parse_ipv4_interfaces(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.contains(" lo ") {
                return None;
            }

            let ifname = line
                .split_once(':')
                .map(|(_, rest)| rest)
                .and_then(|rest| rest.split_whitespace().next())
                .map(str::trim)
                .filter(|value| !value.is_empty())?;

            let mut parts = line.split_whitespace();
            while let Some(part) = parts.next() {
                if part == "inet" {
                    let ip = parts.next()?.split('/').next()?.trim();
                    if !ip.is_empty() && !ip.starts_with("127.") {
                        return Some((ifname.to_string(), ip.to_string()));
                    }
                }
            }
            None
        })
        .collect()
}

pub(super) fn parse_device_types(output: &str) -> HashMap<String, StatusInterfaceKind> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let (device, kind) = line.split_once(':')?;
            let device = device.trim();
            if device.is_empty() || device == "lo" {
                return None;
            }

            let kind = match kind.trim() {
                "wifi" | "wireless" => StatusInterfaceKind::Wifi,
                "ethernet" => StatusInterfaceKind::Ethernet,
                _ => StatusInterfaceKind::Other,
            };

            Some((device.to_string(), kind))
        })
        .collect()
}

pub(super) fn build_status_interfaces(
    raw_interfaces: Vec<(String, String)>,
    device_types: HashMap<String, StatusInterfaceKind>,
) -> Vec<StatusInterfaceIpv4> {
    let mut interfaces = raw_interfaces
        .into_iter()
        .map(|(ifname, ipv4)| StatusInterfaceIpv4 {
            kind: device_types
                .get(&ifname)
                .cloned()
                .unwrap_or(StatusInterfaceKind::Other),
            ifname,
            ipv4,
        })
        .collect::<Vec<_>>();

    interfaces.sort_by(|left, right| {
        status_interface_sort_rank(&left.kind)
            .cmp(&status_interface_sort_rank(&right.kind))
            .then_with(|| left.ifname.cmp(&right.ifname))
    });

    interfaces
}

fn status_interface_sort_rank(kind: &StatusInterfaceKind) -> u8 {
    match kind {
        StatusInterfaceKind::Wifi => 0,
        StatusInterfaceKind::Ethernet => 1,
        StatusInterfaceKind::Other => 2,
    }
}

pub(super) fn preferred_ipv4(interfaces: &[StatusInterfaceIpv4]) -> Option<String> {
    interfaces
        .iter()
        .find(|interface| interface.kind == StatusInterfaceKind::Wifi)
        .or_else(|| interfaces.first())
        .map(|interface| interface.ipv4.clone())
}

pub(super) fn parse_preferred_login_user(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        let user = line.split_whitespace().next()?.trim();
        if user.is_empty() || user == "root" {
            return None;
        }
        Some(user.to_string())
    })
}

pub(super) async fn run_ifconfig(ifname: Option<&str>, timeout_sec: f64) -> SystemExecResult {
    let mut cmd = vec!["ifconfig"];
    if let Some(ifname) = ifname {
        cmd.push(ifname);
    }
    run_system_command(cmd, timeout_sec).await
}
