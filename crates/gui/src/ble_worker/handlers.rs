use client::prepare_request;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tracing::info;

use super::events::{
    command_response_events, command_sent_log, connect_started_log, disconnect_success_detail,
    handshake_completed_log, manual_disconnect_log, request_success_detail,
};
use super::state::WorkerState;
use crate::app::model::{ActionSlot, DisconnectReason, UiEvent};

pub(super) async fn handle_connect_to_candidate(
    ui_tx: &Sender<UiEvent>,
    state: &mut WorkerState,
    name: String,
) {
    emit(
        ui_tx,
        UiEvent::ActionStarted {
            slot: ActionSlot::Connect,
            request_id: None,
        },
    );
    let (client, device) = match state.take_connection_target(&name) {
        Ok(target) => target,
        Err(err) => {
            emit(
                ui_tx,
                UiEvent::ActionFailed {
                    slot: ActionSlot::Connect,
                    request_id: None,
                    error: err.clone(),
                },
            );
            emit(ui_tx, UiEvent::Error(err));
            return;
        }
    };

    emit(ui_tx, UiEvent::ConnectingToCandidate(name.clone()));
    emit(ui_tx, UiEvent::Log(connect_started_log(&device.info.name)));

    match client.connect_session(device).await {
        Ok(session) => {
            let connected_name = session.device_name().to_string();
            state.activate_session(session);
            emit(ui_tx, UiEvent::ConnectedDeviceSelected(name));
            emit(
                ui_tx,
                UiEvent::ActionSucceeded {
                    slot: ActionSlot::Connect,
                    request_id: None,
                    detail: Some(connected_name),
                },
            );
            emit(ui_tx, UiEvent::Log(handshake_completed_log()));
        }
        Err(err) => {
            state.reset_to_idle();
            emit(
                ui_tx,
                UiEvent::ActionFailed {
                    slot: ActionSlot::Connect,
                    request_id: None,
                    error: format!("Connect failed: {}", err),
                },
            );
            emit(
                ui_tx,
                UiEvent::ConnectionFailed(format!("Connect failed: {}", err)),
            );
        }
    }
}

pub(super) async fn handle_disconnect(ui_tx: &Sender<UiEvent>, state: &mut WorkerState) {
    let Some(session) = state.take_active_session() else {
        return;
    };
    emit(
        ui_tx,
        UiEvent::ActionStarted {
            slot: ActionSlot::Disconnect,
            request_id: None,
        },
    );
    let device_name = session.device_name().to_string();
    state.reset_to_idle();
    emit(
        ui_tx,
        UiEvent::Disconnected {
            reason: DisconnectReason::Manual,
        },
    );
    emit(ui_tx, UiEvent::Log(manual_disconnect_log(&device_name)));
    let disconnect_result =
        tokio::time::timeout(Duration::from_secs(2), session.disconnect()).await;
    match disconnect_result {
        Ok(Ok(_)) => emit(
            ui_tx,
            UiEvent::ActionSucceeded {
                slot: ActionSlot::Disconnect,
                request_id: None,
                detail: disconnect_success_detail(&device_name),
            },
        ),
        Ok(Err(err)) => {
            emit(
                ui_tx,
                UiEvent::ActionFailed {
                    slot: ActionSlot::Disconnect,
                    request_id: None,
                    error: format!("Disconnect fail: {}", err),
                },
            );
            emit(ui_tx, UiEvent::Error(format!("Disconnect fail: {}", err)));
        }
        Err(_) => emit(
            ui_tx,
            UiEvent::Error("Disconnect fail: timed out after 2s".to_string()),
        ),
    }
}

pub(super) async fn handle_send_command(
    ui_tx: &Sender<UiEvent>,
    state: &mut WorkerState,
    slot: ActionSlot,
    payload: protocol::requests::CommandPayload,
) {
    let Some(session) = state.active_session_mut() else {
        emit(
            ui_tx,
            UiEvent::ActionFailed {
                slot,
                request_id: None,
                error: "Not connected".to_string(),
            },
        );
        emit(ui_tx, UiEvent::Error("Not connected".to_string()));
        return;
    };

    let request = match prepare_request(payload) {
        Ok(request) => request,
        Err(err) => {
            emit(
                ui_tx,
                UiEvent::ActionFailed {
                    slot,
                    request_id: None,
                    error: format!("Encode fail: {}", err),
                },
            );
            emit(ui_tx, UiEvent::Error(format!("Encode fail: {}", err)));
            return;
        }
    };

    let command_name = request.request.payload.command_name();
    emit(
        ui_tx,
        UiEvent::ActionStarted {
            slot,
            request_id: Some(request.request.id.clone()),
        },
    );
    emit(
        ui_tx,
        UiEvent::Log(command_sent_log(command_name, &request.request.id)),
    );

    if let Err(err) = session.send_request(&request).await {
        emit(
            ui_tx,
            UiEvent::ActionFailed {
                slot,
                request_id: Some(request.request.id.clone()),
                error: format!("Write fail: {}", err),
            },
        );
        emit(ui_tx, UiEvent::Error(format!("Write fail: {}", err)));
        return;
    }

    match session.next_response(30).await {
        Ok(response) => {
            info!(
                device_name = %session.device_name(),
                rssi = ?session.device_rssi(),
                cmd = %command_name,
                request_id = %request.request.id,
                response_id = %response.id,
                "gui.command.completed"
            );
            match command_response_events(&request.request.payload, &response) {
                Ok(events) => {
                    for event in events {
                        emit(ui_tx, event);
                    }
                    if response.ok {
                        match request_success_detail(slot, &request.request.payload, &response) {
                            Ok(detail) => emit(
                                ui_tx,
                                UiEvent::ActionSucceeded {
                                    slot,
                                    request_id: Some(request.request.id.clone()),
                                    detail,
                                },
                            ),
                            Err(err) => {
                                emit(
                                    ui_tx,
                                    UiEvent::ActionFailed {
                                        slot,
                                        request_id: Some(request.request.id.clone()),
                                        error: err.to_string(),
                                    },
                                );
                                emit(ui_tx, UiEvent::Error(err.to_string()));
                            }
                        }
                    } else {
                        emit(
                            ui_tx,
                            UiEvent::ActionFailed {
                                slot,
                                request_id: Some(request.request.id.clone()),
                                error: response.text.clone(),
                            },
                        );
                    }
                }
                Err(err) => {
                    emit(
                        ui_tx,
                        UiEvent::ActionFailed {
                            slot,
                            request_id: Some(request.request.id.clone()),
                            error: err.to_string(),
                        },
                    );
                    emit(ui_tx, UiEvent::Error(err.to_string()));
                }
            }
        }
        Err(err) => {
            emit(
                ui_tx,
                UiEvent::ActionFailed {
                    slot,
                    request_id: Some(request.request.id.clone()),
                    error: format!("Read fail: {}", err),
                },
            );
            emit(ui_tx, UiEvent::Error(format!("Read fail: {}", err)));
        }
    }
}

pub(super) fn emit(ui_tx: &Sender<UiEvent>, event: UiEvent) {
    let _ = ui_tx.send(event);
}
