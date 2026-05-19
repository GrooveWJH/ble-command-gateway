#[cfg(target_os = "linux")]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
#[cfg(target_os = "linux")]
use tokio::sync::{broadcast, Mutex};
#[cfg(target_os = "linux")]
use tracing::{info, warn};

#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct CommandEventSender {
    tx: broadcast::Sender<Vec<u8>>,
    foreground_lock: Arc<Mutex<()>>,
    service_context: crate::services::ServiceContext,
}

#[cfg(target_os = "linux")]
impl CommandEventSender {
    pub fn new(
        tx: broadcast::Sender<Vec<u8>>,
        service_context: crate::services::ServiceContext,
    ) -> Self {
        Self {
            tx,
            foreground_lock: Arc::new(Mutex::new(())),
            service_context,
        }
    }

    pub async fn handle_request(&self, req: protocol::CommandRequest, command_name: String) {
        if is_long_running(&req.payload) {
            self.spawn_long_running(req, command_name);
        } else {
            let result =
                crate::services::run_payload_command(&self.service_context, &req.payload, 30.0)
                    .await;
            self.send_response_event(
                result_response(&req, &command_name, 1, result),
                &command_name,
            );
        }
    }

    fn spawn_long_running(&self, req: protocol::CommandRequest, command_name: String) {
        let tx = self.tx.clone();
        let lock = self.foreground_lock.clone();
        let service_context = self.service_context.clone();
        tokio::spawn(async move {
            let Ok(_guard) = lock.try_lock() else {
                send_response_event(
                    &tx,
                    protocol::CommandResponse::result(
                        req.id,
                        Some(command_name.clone()),
                        false,
                        protocol::codes::CODE_BUSY,
                        "another foreground command is already running",
                        None,
                    ),
                    &command_name,
                );
                return;
            };

            send_response_event(
                &tx,
                protocol::CommandResponse::accepted(
                    req.id.clone(),
                    command_name.clone(),
                    "accepted",
                    None,
                ),
                &command_name,
            );

            let next_seq = Arc::new(AtomicU64::new(2));
            let progress_task = spawn_progress_loop(
                tx.clone(),
                req.id.clone(),
                command_name.clone(),
                next_seq.clone(),
            );
            let result =
                crate::services::run_payload_command(&service_context, &req.payload, 30.0).await;
            progress_task.abort();
            let final_seq = next_seq.fetch_add(1, Ordering::SeqCst);

            send_response_event(
                &tx,
                result_response(&req, &command_name, final_seq, result),
                &command_name,
            );
        });
    }

    pub fn send_response_event(&self, resp: protocol::CommandResponse, command_name: &str) {
        send_response_event(&self.tx, resp, command_name);
    }
}

#[cfg(target_os = "linux")]
fn spawn_progress_loop(
    tx: broadcast::Sender<Vec<u8>>,
    request_id: String,
    command_name: String,
    next_seq: Arc<AtomicU64>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let seq = next_seq.fetch_add(1, Ordering::SeqCst);
            send_response_event(
                &tx,
                protocol::CommandResponse::progress(
                    request_id.clone(),
                    command_name.clone(),
                    seq,
                    "please wait",
                    None,
                ),
                &command_name,
            );
        }
    })
}

pub fn is_long_running(payload: &protocol::requests::CommandPayload) -> bool {
    matches!(
        payload,
        protocol::requests::CommandPayload::WifiScan { .. }
            | protocol::requests::CommandPayload::WifiProvision { .. }
            | protocol::requests::CommandPayload::WifiProfilesDelete { .. }
    )
}

#[cfg(target_os = "linux")]
fn result_response(
    req: &protocol::CommandRequest,
    command_name: &str,
    seq: u64,
    result: crate::services::SystemExecResult,
) -> protocol::CommandResponse {
    let mut response = protocol::CommandResponse::result(
        req.id.clone(),
        Some(command_name.to_string()),
        result.ok,
        result.code,
        result.text,
        result.data,
    );
    response.seq = seq;
    response
}

#[cfg(target_os = "linux")]
fn send_response_event(
    tx: &broadcast::Sender<Vec<u8>>,
    resp: protocol::CommandResponse,
    command_name: &str,
) {
    let response_code = resp.code.clone();
    let response_ok = resp.ok;
    let response_bytes = protocol::encode_response(&resp)
        .map(|value| value.len())
        .ok();
    let chunks = protocol::chunking::chunk_response(resp.clone());
    let chunk_count = chunks.len();
    let chunk_mode = if chunk_count > 1 {
        crate::log_view::ChunkMode::ResponseJson
    } else {
        crate::log_view::ChunkMode::Single
    };
    let mut chunk_sizes = Vec::with_capacity(chunk_count);

    for chunk in chunks {
        match protocol::encode_response(&chunk) {
            Ok(ser) => {
                chunk_sizes.push(ser.len());
                let _ = tx.send(ser);
            }
            Err(err) => {
                warn!(
                    request_id = %resp.id,
                    cmd = %command_name,
                    error = %err,
                    "ble.response.encode_failed"
                );
            }
        }
    }
    let max_chunk_bytes = chunk_sizes.iter().copied().max().unwrap_or(0);

    info!(
        request_id = %resp.id,
        cmd = %command_name,
        response_code = %response_code,
        response_ok,
        phase = ?resp.phase,
        seq = resp.seq,
        final_flag = resp.final_flag,
        chunk_count,
        chunk_mode = chunk_mode.as_str(),
        chunk_sizes = ?chunk_sizes,
        payload_limit = protocol::config::MAX_BLE_PAYLOAD_BYTES,
        response_bytes,
        max_chunk_bytes,
        "ble.response.sent"
    );
    crate::log_view::emit_block(&crate::log_view::response_block(
        &crate::log_view::ResponseLogView {
            request_id: &resp.id,
            command_name,
            response_code: &response_code,
            response_ok,
            response_bytes,
            payload_limit: protocol::config::MAX_BLE_PAYLOAD_BYTES,
            chunk_mode,
            chunk_sizes: &chunk_sizes,
        },
    ));
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_slow_foreground_commands_emit_progress_events() {
        assert!(super::is_long_running(
            &protocol::requests::CommandPayload::WifiScan { ifname: None }
        ));
        assert!(super::is_long_running(
            &protocol::requests::CommandPayload::WifiProvision {
                ssid: "LabWiFi".to_string(),
                pwd: None,
            }
        ));
        assert!(super::is_long_running(
            &protocol::requests::CommandPayload::WifiProfilesDelete {
                uuids: vec!["uuid".to_string()],
                force: false,
            }
        ));
        assert!(!super::is_long_running(
            &protocol::requests::CommandPayload::LinkHeartbeat
        ));
        assert!(!super::is_long_running(
            &protocol::requests::CommandPayload::WifiProfilesList
        ));
    }
}
