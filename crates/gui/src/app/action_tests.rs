use super::model::{
    latest_feedback_for_slots, ActionPhase, ActionSlot, AppModel, ProvisionResultCard, UiEvent,
};
use super::reducer::reduce;

#[test]
fn action_started_marks_model_busy_without_clearing_previous_result_cards() {
    let mut model = AppModel {
        provision_result: Some(ProvisionResultCard {
            ok: true,
            code: "PROVISION_SUCCESS".to_string(),
            status: "Connected".to_string(),
            ssid: "LabWiFi".to_string(),
            ip: Some("192.168.10.2".to_string()),
            text: "connected".to_string(),
        }),
        ..AppModel::default()
    };

    reduce(
        &mut model,
        UiEvent::ActionStarted {
            slot: ActionSlot::Provision,
            request_id: Some("req-1".to_string()),
        },
    );

    assert_eq!(model.active_action, Some(ActionSlot::Provision));
    let feedback = model.action_feedback.get(&ActionSlot::Provision).unwrap();
    assert_eq!(feedback.phase, ActionPhase::Running);
    assert_eq!(feedback.request_id.as_deref(), Some("req-1"));
    assert!(model.provision_result.is_some());
}

#[test]
fn action_succeeded_clears_busy_state_and_records_summary() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::ActionStarted {
            slot: ActionSlot::Status,
            request_id: Some("req-2".to_string()),
        },
    );
    reduce(
        &mut model,
        UiEvent::ActionSucceeded {
            slot: ActionSlot::Status,
            request_id: Some("req-2".to_string()),
            detail: Some("status-updated".to_string()),
        },
    );

    assert!(model.active_action.is_none());
    let feedback = model.action_feedback.get(&ActionSlot::Status).unwrap();
    assert_eq!(feedback.phase, ActionPhase::Succeeded);
    assert_eq!(feedback.detail.as_deref(), Some("status-updated"));
}

#[test]
fn action_failed_clears_busy_state_and_preserves_error_text() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::ActionStarted {
            slot: ActionSlot::WifiScan,
            request_id: Some("req-3".to_string()),
        },
    );
    reduce(
        &mut model,
        UiEvent::ActionFailed {
            slot: ActionSlot::WifiScan,
            request_id: Some("req-3".to_string()),
            error: "Timed out".to_string(),
        },
    );

    assert!(model.active_action.is_none());
    let feedback = model.action_feedback.get(&ActionSlot::WifiScan).unwrap();
    assert_eq!(feedback.phase, ActionPhase::Failed);
    assert_eq!(feedback.error.as_deref(), Some("Timed out"));
}

#[test]
fn latest_feedback_prefers_most_recent_slot_within_same_panel() {
    let mut model = AppModel::default();

    reduce(
        &mut model,
        UiEvent::ActionSucceeded {
            slot: ActionSlot::Status,
            request_id: Some("req-4".to_string()),
            detail: Some("status".to_string()),
        },
    );
    reduce(
        &mut model,
        UiEvent::ActionSucceeded {
            slot: ActionSlot::Ping,
            request_id: Some("req-5".to_string()),
            detail: Some("ping".to_string()),
        },
    );

    let feedback = latest_feedback_for_slots(&model, &[ActionSlot::Status, ActionSlot::Ping])
        .expect("latest feedback should exist");
    assert_eq!(feedback.slot, ActionSlot::Ping);
}
