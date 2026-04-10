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
