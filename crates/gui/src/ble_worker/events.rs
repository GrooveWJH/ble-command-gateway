use anyhow::Result;
use client::ScanProgressEvent;

use crate::app::model::{
    ActionSlot, CommandResultSummary, DiagnosticResultCard, ProvisionResultCard, UiEvent,
};

pub(super) fn command_response_events(
    payload: &protocol::requests::CommandPayload,
    response: &protocol::CommandResponse,
) -> Result<Vec<UiEvent>> {
    let mut events = Vec::new();

    if matches!(payload, protocol::requests::CommandPayload::WifiScan { .. }) {
        let data: protocol::responses::WifiScanResponseData = response.decode_data()?;
        events.push(UiEvent::WifiScanLoaded(data.networks));
    }
    if let Some(result_card) = diagnostic_result(payload, response)? {
        events.push(UiEvent::DiagnosticResult(result_card));
    }
    if let Some(result_card) = provision_result(payload, response)? {
        events.push(UiEvent::ProvisionResult(result_card));
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

fn provision_result(
    payload: &protocol::requests::CommandPayload,
    response: &protocol::CommandResponse,
) -> Result<Option<ProvisionResultCard>> {
    if !matches!(
        payload,
        protocol::requests::CommandPayload::Provision { .. }
    ) {
        return Ok(None);
    }

    let data: protocol::responses::ProvisionResponseData = response.decode_data()?;
    Ok(Some(ProvisionResultCard {
        ok: response.ok,
        code: response.code.clone(),
        status: format!("{:?}", data.status),
        ssid: data.ssid,
        ip: data.ip,
        text: response.text.clone(),
    }))
}

fn diagnostic_result(
    payload: &protocol::requests::CommandPayload,
    response: &protocol::CommandResponse,
) -> Result<Option<DiagnosticResultCard>> {
    match payload {
        protocol::requests::CommandPayload::Status => {
            let data: protocol::responses::StatusResponseData = response.decode_data()?;
            let network = data.network.unwrap_or_else(|| "Not connected".to_string());
            let ip = data.ip.unwrap_or_else(|| "Unavailable".to_string());
            Ok(Some(DiagnosticResultCard {
                title: "System Status".to_string(),
                ok: response.ok,
                code: response.code.clone(),
                lines: vec![
                    format!("Hostname: {}", data.hostname),
                    format!("System: {}", data.system),
                    format!("User: {}", data.user),
                    format!("Network: {}", network),
                    format!("IP: {}", ip),
                ],
            }))
        }
        protocol::requests::CommandPayload::Ping => {
            let data = response
                .decode_data::<protocol::responses::PingResponseData>()
                .ok();
            let pong = data.map(|value| value.pong).unwrap_or(response.ok);
            Ok(Some(DiagnosticResultCard {
                title: "Ping Test".to_string(),
                ok: response.ok,
                code: response.code.clone(),
                lines: vec![format!(
                    "Reachability: {}",
                    if pong { "pong" } else { "failed" }
                )],
            }))
        }
        protocol::requests::CommandPayload::Help => {
            let data: protocol::responses::HelpResponseData = response.decode_data()?;
            let commands = if data.commands.is_empty() {
                "none".to_string()
            } else {
                data.commands.join(", ")
            };
            Ok(Some(DiagnosticResultCard {
                title: "Remote Help".to_string(),
                ok: response.ok,
                code: response.code.clone(),
                lines: vec![
                    format!("Supported commands: {}", data.commands.len()),
                    commands,
                ],
            }))
        }
        _ => Ok(None),
    }
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

pub(super) fn scan_completed_detail(candidate_count: usize) -> Option<String> {
    Some(candidate_count.to_string())
}

pub(super) fn connect_started_log(device_name: &str) -> String {
    format!("[SYS] Connecting to {}...", device_name)
}

pub(super) fn connect_selected_log(device_name: &str) -> String {
    format!(
        "[SYS] Candidate selected, stopping scan and connecting to {}...",
        device_name
    )
}

pub(super) fn handshake_completed_log() -> String {
    "[SYS] Handshake complete. MTU synced.".to_string()
}

pub(super) fn scan_stopped_log() -> String {
    "[SYS] Scan stopped by user.".to_string()
}

pub(super) fn manual_disconnect_log(device_name: &str) -> String {
    format!("[SYS] Disconnected from {}.", device_name)
}

pub(super) fn disconnect_success_detail(device_name: &str) -> Option<String> {
    Some(device_name.to_string())
}

pub(super) fn heartbeat_disconnected_log(device_name: &str, failures: u8) -> String {
    format!(
        "[ERR] Heartbeat failed {failures} times for {device_name}, connection marked as disconnected."
    )
}

pub(super) fn command_sent_log(command_name: &str, request_id: &str) -> String {
    format!(">> TX CMD: {command_name} ({request_id})")
}

pub(super) fn response_log_line(response: &protocol::CommandResponse) -> String {
    format!("<< RX {} {}: {}", response.id, response.code, response.text)
}

pub(super) fn raw_payload_log(payload: &str) -> String {
    format!(">> TX RAW: {}", payload.trim())
}

pub(super) fn raw_payload_success_detail() -> Option<String> {
    Some("written".to_string())
}

pub(super) fn request_success_detail(
    slot: ActionSlot,
    _payload: &protocol::requests::CommandPayload,
    response: &protocol::CommandResponse,
) -> Result<Option<String>> {
    match slot {
        ActionSlot::WifiScan => {
            let data: protocol::responses::WifiScanResponseData = response.decode_data()?;
            Ok(Some(data.networks.len().to_string()))
        }
        ActionSlot::Provision => {
            let data: protocol::responses::ProvisionResponseData = response.decode_data()?;
            Ok(Some(format!("{:?}", data.status)))
        }
        ActionSlot::Status => {
            let data: protocol::responses::StatusResponseData = response.decode_data()?;
            Ok(Some(data.hostname))
        }
        ActionSlot::Help => {
            let data: protocol::responses::HelpResponseData = response.decode_data()?;
            Ok(Some(data.commands.len().to_string()))
        }
        ActionSlot::Ping => Ok(Some(response.code.clone())),
        _ => Ok(None),
    }
}
