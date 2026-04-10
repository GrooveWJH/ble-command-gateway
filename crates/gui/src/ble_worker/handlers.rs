use anyhow::Result;
use client::{prepare_request, BleClient, ScanProgressEvent};
use std::sync::mpsc::Sender;
use tracing::info;

use super::WorkerState;
use crate::app::model::{CommandResultSummary, UiEvent};

pub(super) async fn handle_scan_candidates(
    ui_tx: &Sender<UiEvent>,
    state: &mut WorkerState,
    prefix: String,
    timeout_secs: u64,
) {
    emit(ui_tx, UiEvent::ScanStarted);
    emit(ui_tx, UiEvent::Log(scan_started_log(&prefix)));

    match BleClient::new().await {
        Ok(client) => {
            let mut named_device_count = 0usize;

            match client
                .scan_candidates_with_progress(&prefix, timeout_secs, |event| {
                    named_device_count += 1;
                    emit(ui_tx, UiEvent::Log(scan_progress_log(&event)));
                })
                .await
            {
                Ok(devices) => {
                    let infos = devices
                        .iter()
                        .map(|device| device.info.clone())
                        .collect::<Vec<_>>();
                    state.store_scan_results(client, devices);
                    emit(
                        ui_tx,
                        UiEvent::Log(scan_completed_log(named_device_count, infos.len())),
                    );
                    emit(ui_tx, UiEvent::ScanResults(infos));
                }
                Err(err) => emit(ui_tx, UiEvent::Error(err.to_string())),
            }
        }
        Err(err) => emit(ui_tx, UiEvent::Error(err.to_string())),
    }
}

pub(super) async fn handle_connect_to_candidate(
    ui_tx: &Sender<UiEvent>,
    state: &mut WorkerState,
    name: String,
) {
    let (client, device) = match state.take_connection_target(&name) {
        Ok(target) => target,
        Err(err) => {
            emit(ui_tx, UiEvent::Error(err));
            return;
        }
    };

    emit(ui_tx, UiEvent::Log(connect_started_log(&device.info.name)));

    match client.connect_session(device).await {
        Ok(session) => {
            state.activate_session(session);
            emit(ui_tx, UiEvent::ConnectedDeviceSelected(name));
            emit(ui_tx, UiEvent::Log(handshake_completed_log()));
        }
        Err(err) => {
            state.restore_client(client);
            emit(ui_tx, UiEvent::Error(format!("Connect failed: {}", err)));
        }
    }
}

pub(super) async fn handle_send_command(
    ui_tx: &Sender<UiEvent>,
    state: &mut WorkerState,
    payload: protocol::requests::CommandPayload,
) {
    let Some(session) = state.active_session_mut() else {
        emit(ui_tx, UiEvent::Error("Not connected".to_string()));
        return;
    };

    let request = match prepare_request(payload) {
        Ok(request) => request,
        Err(err) => {
            emit(ui_tx, UiEvent::Error(format!("Encode fail: {}", err)));
            return;
        }
    };

    let command_name = request.request.payload.command_name();
    emit(
        ui_tx,
        UiEvent::Log(command_sent_log(command_name, &request.request.id)),
    );

    if let Err(err) = session.send_request(&request).await {
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
                }
                Err(err) => emit(ui_tx, UiEvent::Error(err.to_string())),
            }
        }
        Err(err) => emit(ui_tx, UiEvent::Error(format!("Read fail: {}", err))),
    }
}

pub(super) async fn handle_send_raw(
    ui_tx: &Sender<UiEvent>,
    state: &mut WorkerState,
    payload: String,
) {
    let Some(session) = state.active_session() else {
        emit(ui_tx, UiEvent::Error("Not connected".to_string()));
        return;
    };

    info!(
        device_name = %session.device_name(),
        rssi = ?session.device_rssi(),
        payload_bytes = payload.len(),
        "gui.raw_payload.sent"
    );
    emit(ui_tx, UiEvent::Log(raw_payload_log(&payload)));

    if let Err(err) = session.send_payload(payload.as_bytes()).await {
        emit(ui_tx, UiEvent::Error(format!("Write fail: {}", err)));
    }
}

pub(super) fn command_response_events(
    payload: &protocol::requests::CommandPayload,
    response: &protocol::CommandResponse,
) -> Result<Vec<UiEvent>> {
    let mut events = Vec::new();

    if matches!(payload, protocol::requests::CommandPayload::WifiScan { .. }) {
        let data: protocol::responses::WifiScanResponseData = response.decode_data()?;
        events.push(UiEvent::WifiScanLoaded(data.networks));
    }

    events.push(UiEvent::CommandCompleted(CommandResultSummary {
        request_id: response.id.clone(),
        code: response.code.clone(),
        text: response.text.clone(),
        ok: response.ok,
    }));
    events.push(UiEvent::Log(response_log_line(response)));

    Ok(events)
}

pub(super) fn scan_started_log(prefix: &str) -> String {
    format!("[SYS] Scanning for '{}'...", prefix)
}

pub(super) fn scan_progress_log(event: &ScanProgressEvent) -> String {
    let signal = event
        .rssi
        .map(|value| format!("{value} dBm"))
        .unwrap_or_else(|| "RSSI unknown".to_string());
    let prefix = if event.matches_prefix {
        "[SCAN][MATCH]"
    } else {
        "[SCAN]"
    };
    format!("{prefix} {} ({signal})", event.device_name)
}

pub(super) fn scan_completed_log(named_device_count: usize, candidate_count: usize) -> String {
    format!(
        "[SYS] Found {named_device_count} named device(s); {candidate_count} candidate device(s) match the prefix."
    )
}

pub(super) fn connect_started_log(device_name: &str) -> String {
    format!("[SYS] Connecting to {}...", device_name)
}

pub(super) fn handshake_completed_log() -> String {
    "[SYS] Handshake complete. MTU synced.".to_string()
}

pub(super) fn command_sent_log(command_name: &str, request_id: &str) -> String {
    format!(">> TX CMD: {} ({})", command_name, request_id)
}

pub(super) fn response_log_line(response: &protocol::CommandResponse) -> String {
    format!("<< RX {} {}: {}", response.id, response.code, response.text)
}

pub(super) fn raw_payload_log(payload: &str) -> String {
    format!(">> TX RAW: {}", payload)
}

fn emit(ui_tx: &Sender<UiEvent>, event: UiEvent) {
    let _ = ui_tx.send(event);
}
