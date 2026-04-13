use anyhow::{bail, Result};
use chrono::Local;
use client::prepare_request;
use std::sync::mpsc::Sender;

use super::events::heartbeat_disconnected_log;
use super::handlers::emit;
use super::state::WorkerState;
use crate::app::model::{DisconnectReason, UiEvent};

const HEARTBEAT_FAILURE_LIMIT: u8 = 3;
const HEARTBEAT_RESPONSE_TIMEOUT_SECS: u64 = 3;

pub(super) async fn handle_heartbeat(ui_tx: &Sender<UiEvent>, state: &mut WorkerState) {
    let Some((device_name, heartbeat_result)) = run_heartbeat(state).await else {
        return;
    };

    match heartbeat_result {
        Ok(at) => {
            state.reset_heartbeat_failures();
            emit(ui_tx, UiEvent::HeartbeatOk { at });
        }
        Err(_) => {
            let failures = state.record_heartbeat_failure();
            if failures < HEARTBEAT_FAILURE_LIMIT {
                emit(ui_tx, UiEvent::HeartbeatMissed(failures));
                return;
            }

            let disconnect_error = match state.take_active_session() {
                Some(session) => session.disconnect().await.err(),
                None => None,
            };
            state.reset_to_idle();
            emit(
                ui_tx,
                UiEvent::Log(heartbeat_disconnected_log(&device_name, failures)),
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
