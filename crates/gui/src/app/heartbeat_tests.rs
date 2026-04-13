use super::model::{
    AppModel, DiagnosticResultCard, DisconnectReason, ProvisionResultCard, UiEvent,
};
use super::reducer::reduce;

#[test]
fn heartbeat_ok_resets_failure_counter_and_records_timestamp() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::ConnectedDeviceSelected("Yundrone_UAV-15-19-A7".to_string()),
    );
    reduce(&mut model, UiEvent::HeartbeatMissed(2));
    reduce(
        &mut model,
        UiEvent::HeartbeatOk {
            at: "12:34:56".to_string(),
        },
    );

    assert!(model.is_connected);
    assert!(model.heartbeat_ok);
    assert_eq!(model.heartbeat_failures, 0);
    assert_eq!(model.last_heartbeat_at.as_deref(), Some("12:34:56"));
}

#[test]
fn heartbeat_disconnect_keeps_existing_result_cards() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::ConnectedDeviceSelected("Yundrone_UAV-15-19-A7".to_string()),
    );
    reduce(
        &mut model,
        UiEvent::DiagnosticResult(DiagnosticResultCard {
            title: "System Status".to_string(),
            ok: true,
            code: "OK".to_string(),
            lines: vec!["Hostname: orangepi4pro".to_string()],
        }),
    );
    reduce(
        &mut model,
        UiEvent::ProvisionResult(ProvisionResultCard {
            ok: true,
            code: "PROVISION_SUCCESS".to_string(),
            status: "Connected".to_string(),
            ssid: "LabWiFi".to_string(),
            ip: Some("192.168.10.2".to_string()),
            text: "connected".to_string(),
        }),
    );
    reduce(&mut model, UiEvent::HeartbeatMissed(2));
    reduce(
        &mut model,
        UiEvent::Disconnected {
            reason: DisconnectReason::HeartbeatFailed,
        },
    );

    assert!(!model.is_connected);
    assert!(!model.is_connecting);
    assert_eq!(model.heartbeat_failures, 0);
    assert!(model.diagnostic_result.is_some());
    assert!(model.provision_result.is_some());
}

#[test]
fn manual_disconnect_returns_to_idle_without_clearing_results() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::ConnectedDeviceSelected("Yundrone_UAV-15-19-A7".to_string()),
    );
    reduce(
        &mut model,
        UiEvent::DiagnosticResult(DiagnosticResultCard {
            title: "Ping Test".to_string(),
            ok: true,
            code: "OK".to_string(),
            lines: vec!["Reachability: pong".to_string()],
        }),
    );
    reduce(
        &mut model,
        UiEvent::Disconnected {
            reason: DisconnectReason::Manual,
        },
    );

    assert!(!model.is_connected);
    assert!(!model.is_connecting);
    assert!(!model.is_scanning);
    assert!(model.connected_device_name.is_none());
    assert!(model.scan_candidates.is_empty());
    assert!(model.diagnostic_result.is_some());
}
