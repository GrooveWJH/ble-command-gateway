use anyhow::{bail, Result};
use chrono::Local;
use client::prepare_request;
use std::sync::mpsc::Sender;
use std::time::Instant;

use super::events::heartbeat_disconnected_log;
use super::handlers::emit;
use super::state::WorkerState;
use crate::app::model::{DisconnectReason, UiEvent};

const HEARTBEAT_RESPONSE_TIMEOUT_SECS: u64 = 3;

pub(super) async fn handle_heartbeat(ui_tx: &Sender<UiEvent>, state: &mut WorkerState) {
    if state.heartbeat_deadline_elapsed(Instant::now()) {
        disconnect_due_to_heartbeat_grace(ui_tx, state).await;
        return;
    }

    let Some((device_name, heartbeat_result)) = run_heartbeat(state).await else {
        return;
    };

    match heartbeat_result {
        Ok(at) => {
            state.reset_heartbeat_failures();
            emit(ui_tx, UiEvent::HeartbeatOk { at });
        }
        Err(_) => {
            let failures = state.record_heartbeat_failure(Instant::now());
            emit(ui_tx, UiEvent::HeartbeatMissed(failures));
            let _ = device_name;
        }
    }
}

async fn disconnect_due_to_heartbeat_grace(ui_tx: &Sender<UiEvent>, state: &mut WorkerState) {
    let Some(session) = state.take_active_session() else {
        state.reset_to_idle();
        return;
    };

    let device_name = session.device_name().to_string();
    let failures = state.heartbeat_failures().max(1);
    let disconnect_error = session.disconnect().await.err();
    state.reset_to_idle();
    emit(
        ui_tx,
        UiEvent::Log(heartbeat_disconnected_log(&device_name, failures, true)),
    );
    emit(
        ui_tx,
        UiEvent::Disconnected {
            reason: DisconnectReason::HeartbeatFailed,
        },
    );
    if let Some(disconnect_error) = disconnect_error {
        emit(
            ui_tx,
            UiEvent::Error(format!("Disconnect fail: {}", disconnect_error)),
        );
    }
}

async fn run_heartbeat(state: &mut WorkerState) -> Option<(String, Result<String>)> {
    let session = state.active_session_mut()?;
    let device_name = session.device_name().to_string();
    let request = match prepare_request(protocol::requests::CommandPayload::Ping) {
        Ok(request) => request,
        Err(err) => return Some((device_name, Err(err))),
    };

    let result = async {
        session.send_request(&request).await?;
        let response = session
            .next_response(HEARTBEAT_RESPONSE_TIMEOUT_SECS)
            .await?;
        if response.id != request.request.id {
            bail!(
                "Heartbeat response id mismatch: expected {}, got {}",
                request.request.id,
                response.id
            );
        }
        if !response.ok {
            bail!("Heartbeat failed: {} {}", response.code, response.text);
        }
        Ok(heartbeat_timestamp())
    }
    .await;

    Some((device_name, result))
}

fn heartbeat_timestamp() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
