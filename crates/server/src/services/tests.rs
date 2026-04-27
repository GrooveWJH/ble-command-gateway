use super::{
    command_runner::{run_command_with_timeout, CommandRunStatus},
    map_run_output,
    network::{finalize_wifi_provision, parse_nmcli_wifi_list},
    run_payload_command, SystemExecResult,
};

#[tokio::test]
async fn help_command_is_supported() {
    let result = run_payload_command(&protocol::requests::CommandPayload::Help, 1.0).await;
    let data: protocol::responses::HelpResponseData =
        protocol::responses::from_map(result.data.as_ref().unwrap()).unwrap();

    assert!(result.ok);
    assert_eq!(result.code, protocol::codes::CODE_OK);
    assert!(!data.commands.is_empty());
}

#[tokio::test]
async fn ping_command_is_supported() {
    let result = run_payload_command(&protocol::requests::CommandPayload::Ping, 1.0).await;

    assert!(result.ok);
    assert_eq!(result.text, "pong");
    assert_eq!(result.code, protocol::codes::CODE_OK);
}

#[tokio::test]
async fn status_command_is_supported() {
    let result = run_payload_command(&protocol::requests::CommandPayload::Status, 1.0).await;
    let data: protocol::responses::StatusResponseData =
        protocol::responses::from_map(result.data.as_ref().unwrap()).unwrap();

    assert!(result.ok);
    assert!(!data.hostname.is_empty());
    assert!(!data.system.is_empty());
    assert!(!data.user.is_empty());
}

#[test]
fn active_network_name_prefers_non_loopback_connections() {
    let output = "\
NAME                DEVICE\n\
LabWiFi             wlan0\n\
lo                  lo\n";

    let network = super::system_commands::parse_active_network_name(output);

    assert_eq!(network.as_deref(), Some("LabWiFi"));
}

#[test]
fn ip_address_parser_prefers_first_non_loopback_ipv4() {
    let output = "\
2: wlan0    inet 192.168.10.2/24 brd 192.168.10.255 scope global dynamic wlan0\n\
3: lo       inet 127.0.0.1/8 scope host lo\n";

    let ip =
        super::system_commands::preferred_ipv4(&super::system_commands::build_status_interfaces(
            super::system_commands::parse_ipv4_interfaces(output),
            std::collections::HashMap::from([(
                "wlan0".to_string(),
                protocol::responses::StatusInterfaceKind::Wifi,
            )]),
        ));

    assert_eq!(ip.as_deref(), Some("192.168.10.2"));
}

#[test]
fn parse_ipv4_interfaces_collects_all_global_addresses() {
    let output = "\
2: eth0    inet 10.24.6.9/24 brd 10.24.6.255 scope global dynamic eth0\n\
3: wlan0    inet 192.168.10.2/24 brd 192.168.10.255 scope global dynamic wlan0\n\
4: lo       inet 127.0.0.1/8 scope host lo\n\
5: docker0 inet 172.17.0.1/16 brd 172.17.255.255 scope global docker0\n";

    let interfaces = super::system_commands::parse_ipv4_interfaces(output);

    assert_eq!(interfaces.len(), 3);
    assert_eq!(interfaces[0], ("eth0".to_string(), "10.24.6.9".to_string()));
    assert_eq!(
        interfaces[1],
        ("wlan0".to_string(), "192.168.10.2".to_string())
    );
    assert_eq!(
        interfaces[2],
        ("docker0".to_string(), "172.17.0.1".to_string())
    );
}

#[test]
fn parse_device_types_maps_wifi_ethernet_and_other_interfaces() {
    let output = "\
wlan0:wifi\n\
eth0:ethernet\n\
docker0:bridge\n\
lo:loopback\n";

    let types = super::system_commands::parse_device_types(output);

    assert_eq!(
        types.get("wlan0"),
        Some(&protocol::responses::StatusInterfaceKind::Wifi)
    );
    assert_eq!(
        types.get("eth0"),
        Some(&protocol::responses::StatusInterfaceKind::Ethernet)
    );
    assert_eq!(
        types.get("docker0"),
        Some(&protocol::responses::StatusInterfaceKind::Other)
    );
}

#[test]
fn build_status_interfaces_sorts_wifi_then_ethernet_then_other() {
    let interfaces = super::system_commands::build_status_interfaces(
        vec![
            ("eth0".to_string(), "10.24.6.9".to_string()),
            ("wlan1".to_string(), "172.16.0.22".to_string()),
            ("docker0".to_string(), "172.17.0.1".to_string()),
            ("wlan0".to_string(), "192.168.10.2".to_string()),
        ],
        std::collections::HashMap::from([
            (
                "wlan0".to_string(),
                protocol::responses::StatusInterfaceKind::Wifi,
            ),
            (
                "wlan1".to_string(),
                protocol::responses::StatusInterfaceKind::Wifi,
            ),
            (
                "eth0".to_string(),
                protocol::responses::StatusInterfaceKind::Ethernet,
            ),
        ]),
    );

    assert_eq!(interfaces.len(), 4);
    assert_eq!(interfaces[0].ifname, "wlan0");
    assert_eq!(interfaces[1].ifname, "wlan1");
    assert_eq!(interfaces[2].ifname, "eth0");
    assert_eq!(interfaces[3].ifname, "docker0");
}

#[test]
fn preferred_ipv4_prefers_wifi_before_other_interfaces() {
    let ip = super::system_commands::preferred_ipv4(&[
        protocol::responses::StatusInterfaceIpv4 {
            ifname: "eth0".to_string(),
            kind: protocol::responses::StatusInterfaceKind::Ethernet,
            ipv4: "10.24.6.9".to_string(),
        },
        protocol::responses::StatusInterfaceIpv4 {
            ifname: "wlan0".to_string(),
            kind: protocol::responses::StatusInterfaceKind::Wifi,
            ipv4: "192.168.10.2".to_string(),
        },
    ]);

    assert_eq!(ip.as_deref(), Some("192.168.10.2"));
}

#[test]
fn current_user_prefers_non_root_ssh_session() {
    let who_output = "\
orangepi pts/0 2026-04-13 10:00 (192.168.10.20)\n\
root     pts/1 2026-04-13 10:01 (192.168.10.30)\n";

    let user = super::system_commands::parse_preferred_login_user(who_output);

    assert_eq!(user.as_deref(), Some("orangepi"));
}

#[tokio::test]
async fn provisioning_requires_ssid() {
    let result = run_payload_command(
        &protocol::requests::CommandPayload::Provision {
            ssid: String::new(),
            pwd: None,
        },
        1.0,
    )
    .await;

    assert!(!result.ok);
    assert_eq!(result.code, protocol::codes::CODE_BAD_REQUEST);
}

#[test]
fn parse_nmcli_wifi_list_skips_invalid_rows() {
    let rows = "\
*:aa:LabWiFi:6:130 Mbit/s:78:▂▄▆_:WPA2\n\
bad-row\n\
:bb::11:130 Mbit/s:40:▂▄__:WPA2\n";

    let entries = parse_nmcli_wifi_list(rows);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].ssid, "LabWiFi");
    assert_eq!(entries[0].channel, "6");
    assert_eq!(entries[0].signal, 78);
}

#[test]
fn parse_nmcli_wifi_list_unescapes_ssid_colons() {
    let rows = r"*:aa:Drone\:Debug:1:130 Mbit/s:40:▂▄__:WPA2";

    let entries = parse_nmcli_wifi_list(rows);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].ssid, "Drone:Debug");
}

#[test]
fn finalize_wifi_provision_success_is_machine_readable() {
    let result = finalize_wifi_provision(
        "LabWiFi",
        SystemExecResult::ok("connected", None),
        Some("192.168.10.2".to_string()),
    );
    let data: protocol::responses::ProvisionResponseData =
        protocol::responses::from_map(result.data.as_ref().unwrap()).unwrap();

    assert!(result.ok);
    assert_eq!(result.code, protocol::codes::CODE_PROVISION_SUCCESS);
    assert_eq!(data.status, protocol::responses::ProvisionState::Connected);
    assert_eq!(data.ip.as_deref(), Some("192.168.10.2"));
}

#[test]
fn finalize_wifi_provision_failure_preserves_timeout_code() {
    let result = finalize_wifi_provision(
        "LabWiFi",
        SystemExecResult::error(protocol::codes::CODE_TIMEOUT, "timed out"),
        None,
    );
    let data: protocol::responses::ProvisionResponseData =
        protocol::responses::from_map(result.data.as_ref().unwrap()).unwrap();

    assert!(!result.ok);
    assert_eq!(result.code, protocol::codes::CODE_TIMEOUT);
    assert_eq!(data.status, protocol::responses::ProvisionState::Failed);
}

#[test]
fn finalize_wifi_provision_failure_uses_provision_fail_for_command_errors() {
    let result = finalize_wifi_provision(
        "LabWiFi",
        SystemExecResult::error(protocol::codes::CODE_INTERNAL_ERROR, "wrong password"),
        None,
    );

    assert!(!result.ok);
    assert_eq!(result.code, protocol::codes::CODE_PROVISION_FAIL);
    assert!(result.text.contains("wrong password"));
}

#[tokio::test]
async fn command_runner_rejects_empty_commands() {
    let output = run_command_with_timeout(vec![], 0.1).await;

    assert_eq!(
        output.status,
        CommandRunStatus::InvalidInput("empty command".to_string())
    );
}

#[tokio::test]
async fn command_runner_uses_stderr_for_failed_commands() {
    let output = run_command_with_timeout(vec!["sh", "-c", "echo boom >&2; exit 5"], 1.0).await;

    assert_eq!(output.status, CommandRunStatus::Failed(5));
    assert_eq!(output.stderr, "boom");
    assert_eq!(output.preferred_text(), "boom");
}

#[tokio::test]
async fn command_runner_reports_timeouts() {
    let output = run_command_with_timeout(vec!["sh", "-c", "sleep 0.2"], 0.05).await;

    assert_eq!(output.status, CommandRunStatus::TimedOut);
}

#[test]
fn map_run_output_translates_timeout_to_protocol_timeout() {
    let result = map_run_output(
        super::command_runner::CommandRunOutput {
            status: CommandRunStatus::TimedOut,
            stdout: String::new(),
            stderr: String::new(),
        },
        0.5,
    );

    assert!(!result.ok);
    assert_eq!(result.code, protocol::codes::CODE_TIMEOUT);
    assert!(result.text.contains("0.5s"));
}
