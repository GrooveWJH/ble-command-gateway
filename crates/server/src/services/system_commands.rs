use super::{run_system_command, SystemExecResult};

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

    let data = protocol::responses::StatusResponseData {
        hostname: hostname.text.clone(),
        system: system.text.clone(),
        user: user.text.clone(),
    };

    SystemExecResult::ok(
        "status collected",
        Some(protocol::responses::to_map(&data).expect("status response serializes")),
    )
}

pub(super) async fn run_effective_user(timeout_sec: f64) -> SystemExecResult {
    if let Ok(user) = std::env::var("SUDO_USER") {
        let user = user.trim();
        if !user.is_empty() {
            let data = protocol::responses::WhoAmIResponseData {
                user: user.to_string(),
            };
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

pub(super) async fn run_ifconfig(ifname: Option<&str>, timeout_sec: f64) -> SystemExecResult {
    let mut cmd = vec!["ifconfig"];
    if let Some(ifname) = ifname {
        cmd.push(ifname);
    }
    run_system_command(cmd, timeout_sec).await
}
