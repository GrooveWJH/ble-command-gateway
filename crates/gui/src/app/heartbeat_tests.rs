use super::model::{
    AppModel, DiagnosticResultCard, DisconnectReason, ProvisionResultCard, UiEvent,
};
use super::reducer::reduce;
use crate::ble_worker::state::WorkerState;
use std::time::{Duration, Instant};

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

#[test]
fn worker_state_sets_disconnect_deadline_from_first_heartbeat_failure() {
    let mut state = WorkerState::default();
    let started = Instant::now();

    let failures = state.record_heartbeat_failure(started);

    assert_eq!(failures, 1);
    assert_eq!(
        state.heartbeat_disconnect_deadline(),
        Some(started + Duration::from_secs(12))
    );
}

#[test]
fn worker_state_keeps_original_disconnect_deadline_during_grace_window() {
    let mut state = WorkerState::default();
    let started = Instant::now();

    state.record_heartbeat_failure(started);
    let deadline = state.heartbeat_disconnect_deadline().unwrap();
    state.record_heartbeat_failure(started + Duration::from_secs(5));

    assert_eq!(state.heartbeat_disconnect_deadline(), Some(deadline));
}
