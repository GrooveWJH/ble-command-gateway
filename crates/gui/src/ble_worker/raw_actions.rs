use std::sync::mpsc::Sender;

use tracing::info;

use super::events::{raw_payload_log, raw_payload_success_detail};
use super::handlers::emit;
use super::state::WorkerState;
use crate::app::model::{ActionSlot, UiEvent};

pub(super) async fn handle_send_raw(
    ui_tx: &Sender<UiEvent>,
    state: &mut WorkerState,
    slot: ActionSlot,
    payload: String,
) {
    let Some(session) = state.active_session() else {
        emit_action_failed(ui_tx, slot, "Not connected".to_string());
        emit(ui_tx, UiEvent::Error("Not connected".to_string()));
        return;
    };

    emit(
        ui_tx,
        UiEvent::ActionStarted {
            slot,
            request_id: None,
        },
    );
    info!(
        device_name = %session.device_name(),
        rssi = ?session.device_rssi(),
        payload_bytes = payload.len(),
        "gui.raw_payload.sent"
    );
    emit(ui_tx, UiEvent::Log(raw_payload_log(&payload)));

    match session.send_payload(payload.as_bytes()).await {
        Ok(_) => emit(
            ui_tx,
            UiEvent::ActionSucceeded {
                slot,
                request_id: None,
                detail: raw_payload_success_detail(),
            },
        ),
        Err(err) => {
            emit_action_failed(ui_tx, slot, format!("Write fail: {}", err));
            emit(ui_tx, UiEvent::Error(format!("Write fail: {}", err)));
        }
    }
}

fn emit_action_failed(ui_tx: &Sender<UiEvent>, slot: ActionSlot, error: String) {
    emit(
        ui_tx,
        UiEvent::ActionFailed {
            slot,
            request_id: None,
            error,
        },
    );
}
