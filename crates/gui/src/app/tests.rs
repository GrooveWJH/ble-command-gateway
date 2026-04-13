use super::model::{
    AppModel, CommandResultSummary, DiagnosticResultCard, ProvisionResultCard, UiEvent,
};
use super::reducer::reduce;
use std::path::PathBuf;

#[test]
fn wifi_scan_loaded_updates_model_without_parsing_logs() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::WifiScanLoaded(vec![protocol::responses::WifiNetwork {
            ssid: "LabWiFi".to_string(),
            channel: "6".to_string(),
            signal: 78,
        }]),
    );

    assert_eq!(model.wifi_list.len(), 1);
    assert_eq!(model.wifi_list[0].ssid, "LabWiFi");
    assert_eq!(model.logs, vec!["[SYS] Init GUI engine..."]);
}

#[test]
fn scan_results_keep_model_disconnected_until_device_is_selected() {
    let mut model = AppModel::default();

    reduce(&mut model, UiEvent::ScanStarted);
    reduce(
        &mut model,
        UiEvent::ScanCandidateDiscovered(client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-A7".to_string(),
            rssi: Some(-48),
        }),
    );
    reduce(&mut model, UiEvent::ScanFinished);

    assert!(!model.is_scanning);
    assert!(!model.is_connected);
    assert_eq!(model.scan_candidates.len(), 1);
    assert_eq!(model.scan_candidates[0].name, "Yundrone_UAV-15-19-A7");
}

#[test]
fn scan_candidate_discovered_appends_without_replacing_existing_items() {
    let mut model = AppModel::default();

    reduce(&mut model, UiEvent::ScanStarted);
    reduce(
        &mut model,
        UiEvent::ScanCandidateDiscovered(client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-A7".to_string(),
            rssi: Some(-48),
        }),
    );
    reduce(
        &mut model,
        UiEvent::ScanCandidateDiscovered(client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-20-B1".to_string(),
            rssi: Some(-60),
        }),
    );

    assert!(model.is_scanning);
    assert!(!model.is_connected);
    assert_eq!(model.scan_candidates.len(), 2);
    assert_eq!(model.scan_candidates[0].name, "Yundrone_UAV-15-19-A7");
    assert_eq!(model.scan_candidates[1].name, "Yundrone_UAV-15-20-B1");
}

#[test]
fn scan_finished_preserves_incremental_candidates() {
    let mut model = AppModel::default();

    reduce(&mut model, UiEvent::ScanStarted);
    reduce(
        &mut model,
        UiEvent::ScanCandidateDiscovered(client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-A7".to_string(),
            rssi: Some(-48),
        }),
    );
    reduce(&mut model, UiEvent::ScanFinished);

    assert!(!model.is_scanning);
    assert_eq!(model.scan_candidates.len(), 1);
    assert_eq!(model.scan_candidates[0].name, "Yundrone_UAV-15-19-A7");
}

#[test]
fn connect_started_sets_connecting_state_until_connected() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::ScanCandidateDiscovered(client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-A7".to_string(),
            rssi: Some(-48),
        }),
    );
    reduce(
        &mut model,
        UiEvent::ConnectingToCandidate("Yundrone_UAV-15-19-A7".to_string()),
    );

    assert!(model.is_connecting);
    assert!(!model.is_connected);
    assert_eq!(
        model.connected_device_name.as_deref(),
        Some("Yundrone_UAV-15-19-A7")
    );
}

#[test]
fn connect_failed_clears_candidates_and_returns_to_idle() {
    let mut model = AppModel::default();

    reduce(&mut model, UiEvent::ScanStarted);
    reduce(
        &mut model,
        UiEvent::ScanCandidateDiscovered(client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-A7".to_string(),
            rssi: Some(-48),
        }),
    );
    reduce(
        &mut model,
        UiEvent::ConnectingToCandidate("Yundrone_UAV-15-19-A7".to_string()),
    );
    reduce(
        &mut model,
        UiEvent::ConnectionFailed("Connect failed".to_string()),
    );

    assert!(!model.is_scanning);
    assert!(!model.is_connecting);
    assert!(!model.is_connected);
    assert!(model.scan_candidates.is_empty());
    assert!(model.connected_device_name.is_none());
}

#[test]
fn scan_stopped_keeps_already_discovered_candidates() {
    let mut model = AppModel::default();

    reduce(&mut model, UiEvent::ScanStarted);
    reduce(
        &mut model,
        UiEvent::ScanCandidateDiscovered(client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-A7".to_string(),
            rssi: Some(-48),
        }),
    );
    reduce(&mut model, UiEvent::ScanStopped);

    assert!(!model.is_scanning);
    assert_eq!(model.scan_candidates.len(), 1);
}

#[test]
fn command_completed_is_logged_as_display_only_event() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::CommandCompleted(CommandResultSummary {
            request_id: "req-1".to_string(),
            code: "OK".to_string(),
            text: "status complete".to_string(),
            ok: true,
        }),
    );

    assert_eq!(
        model.logs.last().unwrap(),
        "[CMD] req-1 OK: status complete"
    );
    assert!(model.wifi_list.is_empty());
}

#[test]
fn diagnostic_result_event_updates_dedicated_panel_state() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::DiagnosticResult(DiagnosticResultCard {
            title: "System Status".to_string(),
            ok: true,
            code: "OK".to_string(),
            lines: vec![
                "Hostname: orangepi4pro".to_string(),
                "System: Linux 6.1".to_string(),
            ],
        }),
    );

    let result = model
        .diagnostic_result
        .as_ref()
        .expect("result should exist");
    assert_eq!(result.title, "System Status");
    assert!(result.ok);
    assert_eq!(result.code, "OK");
    assert_eq!(result.lines.len(), 2);
}

#[test]
fn provision_result_event_updates_provision_panel_state() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::ProvisionResult(ProvisionResultCard {
            ok: true,
            code: "PROVISION_SUCCESS".to_string(),
            status: "Connected".to_string(),
            ssid: "LabWiFi".to_string(),
            ip: Some("192.168.1.23".to_string()),
            text: "connected".to_string(),
        }),
    );

    let result = model
        .provision_result
        .as_ref()
        .expect("result should exist");
    assert!(result.ok);
    assert_eq!(result.ssid, "LabWiFi");
    assert_eq!(result.ip.as_deref(), Some("192.168.1.23"));
}

#[test]
fn header_badge_text_is_ascii_for_font_safe_rendering() {
    assert!(super::model::header_badge_text().is_ascii());
}

#[test]
fn exported_logs_use_single_newlines_without_blank_lines() {
    let model = AppModel {
        logs: vec![
            "[SYS] one".to_string(),
            "[SCAN] two".to_string(),
            "[ERR] three".to_string(),
        ],
        ..AppModel::default()
    };

    assert_eq!(
        super::model::export_logs(&model.logs),
        "[SYS] one\n[SCAN] two\n[ERR] three"
    );
}

#[test]
fn clear_logs_removes_existing_lines() {
    let mut model = AppModel::default();
    model.logs.push("[SCAN] test".to_string());

    super::model::clear_logs(&mut model);

    assert!(model.logs.is_empty());
}

#[test]
fn macos_bundle_declares_bluetooth_usage_description() {
    let plist_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("macos/Info.plist");
    let plist = std::fs::read_to_string(&plist_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", plist_path.display()));

    assert!(plist.contains("CFBundleIdentifier"));
    assert!(plist.contains("CFBundleExecutable"));
    assert!(plist.contains("CFBundlePackageType"));
    assert!(plist.contains("NSBluetoothAlwaysUsageDescription"));
    assert!(plist.contains("Bluetooth"));
}
