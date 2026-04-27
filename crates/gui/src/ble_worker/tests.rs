use super::events::{
    command_response_events, command_sent_log, connect_selected_log, disconnect_success_detail,
    heartbeat_disconnected_log, manual_disconnect_log, raw_payload_log, raw_payload_success_detail,
    request_success_detail, response_log_line, scan_completed_detail, scan_completed_log,
    scan_progress_log, scan_stopped_log,
};
use crate::app::model::{ActionSlot, UiEvent};

#[test]
fn wifi_scan_response_events_include_loaded_networks_and_summary() {
    let response = protocol::CommandResponse::ok(
        "req-1",
        "wifi scan complete",
        Some(
            protocol::responses::to_map(&protocol::responses::WifiScanResponseData {
                ifname: Some("wlan0".to_string()),
                count: 1,
                networks: vec![protocol::responses::WifiNetwork {
                    ssid: "LabWiFi".to_string(),
                    channel: "6".to_string(),
                    signal: 78,
                }],
            })
            .unwrap(),
        ),
    );

    let events = command_response_events(
        &protocol::requests::CommandPayload::WifiScan { ifname: None },
        &response,
    )
    .unwrap();

    assert!(matches!(&events[0], UiEvent::WifiScanLoaded(networks) if networks.len() == 1));
    assert!(
        matches!(&events[1], UiEvent::CommandCompleted(summary) if summary.request_id == "req-1")
    );
    assert!(matches!(&events[2], UiEvent::Log(line) if line.contains("wifi scan complete")));
}

#[test]
fn non_wifi_response_events_skip_wifi_network_loading() {
    let response = protocol::CommandResponse::ok(
        "req-2",
        "status collected",
        Some(
            protocol::responses::to_map(&protocol::responses::StatusResponseData {
                hostname: "orangepi4pro".to_string(),
                system: "Linux 6.1".to_string(),
                user: "orangepi".to_string(),
                network: Some("LabWiFi".to_string()),
                ip: Some("192.168.10.2".to_string()),
                interfaces: vec![
                    protocol::responses::StatusInterfaceIpv4 {
                        ifname: "wlan0".to_string(),
                        kind: protocol::responses::StatusInterfaceKind::Wifi,
                        ipv4: "192.168.10.2".to_string(),
                    },
                    protocol::responses::StatusInterfaceIpv4 {
                        ifname: "eth0".to_string(),
                        kind: protocol::responses::StatusInterfaceKind::Ethernet,
                        ipv4: "10.24.6.9".to_string(),
                    },
                ],
            })
            .unwrap(),
        ),
    );

    let events =
        command_response_events(&protocol::requests::CommandPayload::Status, &response).unwrap();

    assert_eq!(events.len(), 3);
    assert!(matches!(
        &events[0],
        UiEvent::DiagnosticResult(result)
            if result.title == "System Status"
            && result.lines.iter().any(|line| line.contains("Network: LabWiFi"))
            && result.lines.iter().any(|line| line.contains("Preferred IP: 192.168.10.2"))
            && result.lines.iter().any(|line| line.contains("wlan0 [wifi] -> 192.168.10.2"))
            && result.lines.iter().any(|line| line.contains("eth0 [ethernet] -> 10.24.6.9"))
            && result.lines.iter().any(|line| line.contains("User: orangepi"))
    ));
    assert!(matches!(
        &events[1],
        UiEvent::CommandCompleted(summary) if summary.code == "OK"
    ));
    assert!(matches!(&events[2], UiEvent::Log(line) if line == "<< RX req-2 OK: status collected"));
}

#[test]
fn worker_log_helpers_use_consistent_text() {
    assert_eq!(
        scan_completed_log(5, 3),
        "[SYS] Found 5 named device(s); 3 candidate device(s) match the prefix."
    );
    assert_eq!(
        command_sent_log("status", "req-1"),
        ">> TX CMD: status (req-1)"
    );
    assert_eq!(
        raw_payload_log("{\"cmd\":\"ping\"}"),
        ">> TX RAW: {\"cmd\":\"ping\"}"
    );
    assert_eq!(
        response_log_line(&protocol::CommandResponse::ok("req-3", "done", None)),
        "<< RX req-3 OK: done"
    );
}

#[test]
fn provision_response_emits_provision_result_event() {
    let response = protocol::CommandResponse::ok(
        "req-9",
        "connected",
        Some(
            protocol::responses::to_map(&protocol::responses::ProvisionResponseData {
                status: protocol::responses::ProvisionState::Connected,
                ssid: "LabWiFi".to_string(),
                ip: Some("192.168.1.23".to_string()),
            })
            .unwrap(),
        ),
    );

    let events = command_response_events(
        &protocol::requests::CommandPayload::Provision {
            ssid: "LabWiFi".to_string(),
            pwd: Some("12345678".to_string()),
        },
        &response,
    )
    .unwrap();

    assert!(matches!(
        &events[0],
        UiEvent::ProvisionResult(result) if result.ssid == "LabWiFi"
    ));
}

#[test]
fn scan_progress_log_marks_matching_devices() {
    let line = scan_progress_log(&client::ScanProgressEvent {
        device_name: "Yundrone_UAV-15-19-A7".to_string(),
        rssi: Some(-48),
        matches_prefix: true,
    });

    assert_eq!(line, "[SCAN][MATCH] Yundrone_UAV-15-19-A7 (-48 dBm)");
}

#[test]
fn scan_progress_log_handles_unknown_signal() {
    let line = scan_progress_log(&client::ScanProgressEvent {
        device_name: "Beacon_123".to_string(),
        rssi: None,
        matches_prefix: false,
    });

    assert_eq!(line, "[SCAN] Beacon_123 (RSSI unknown)");
}

#[test]
fn scan_control_logs_are_operator_friendly() {
    assert_eq!(scan_stopped_log(), "[SYS] Scan stopped by user.");
    assert_eq!(
        connect_selected_log("Yundrone_UAV-03-17-5433"),
        "[SYS] Candidate selected, stopping scan and connecting to Yundrone_UAV-03-17-5433..."
    );
    assert_eq!(
        manual_disconnect_log("Yundrone_UAV-03-17-5433"),
        "[SYS] Disconnected from Yundrone_UAV-03-17-5433."
    );
    assert_eq!(
        heartbeat_disconnected_log("Yundrone_UAV-03-17-5433", 3, true),
        "[ERR] Heartbeat failed 3 times for Yundrone_UAV-03-17-5433, grace window elapsed and connection was marked as disconnected."
    );
}

#[test]
fn request_success_detail_summarizes_wifi_scan_results() {
    let response = protocol::CommandResponse::ok(
        "req-10",
        "wifi scan complete",
        Some(
            protocol::responses::to_map(&protocol::responses::WifiScanResponseData {
                ifname: Some("wlan0".to_string()),
                count: 2,
                networks: vec![
                    protocol::responses::WifiNetwork {
                        ssid: "LabWiFi".to_string(),
                        channel: "6".to_string(),
                        signal: 78,
                    },
                    protocol::responses::WifiNetwork {
                        ssid: "DroneMesh".to_string(),
                        channel: "11".to_string(),
                        signal: 63,
                    },
                ],
            })
            .unwrap(),
        ),
    );

    let detail = request_success_detail(
        ActionSlot::WifiScan,
        &protocol::requests::CommandPayload::WifiScan { ifname: None },
        &response,
    )
    .unwrap();

    assert_eq!(detail.as_deref(), Some("2"));
}

#[test]
fn action_summary_helpers_cover_device_and_raw_actions() {
    assert_eq!(scan_completed_detail(3), Some("3".to_string()));
    assert_eq!(
        disconnect_success_detail("Yundrone_UAV-03-17-5433"),
        Some("Yundrone_UAV-03-17-5433".to_string())
    );
    assert_eq!(raw_payload_success_detail(), Some("written".to_string()));
}
