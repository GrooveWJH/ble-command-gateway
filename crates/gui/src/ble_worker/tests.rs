use super::handlers::{
    command_response_events, command_sent_log, raw_payload_log, response_log_line,
    scan_completed_log, scan_progress_log,
};
use crate::app::model::UiEvent;

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
    let response = protocol::CommandResponse::ok("req-2", "pong", None);

    let events =
        command_response_events(&protocol::requests::CommandPayload::Ping, &response).unwrap();

    assert_eq!(events.len(), 2);
    assert!(matches!(&events[0], UiEvent::CommandCompleted(summary) if summary.code == "OK"));
    assert!(matches!(&events[1], UiEvent::Log(line) if line == "<< RX req-2 OK: pong"));
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
