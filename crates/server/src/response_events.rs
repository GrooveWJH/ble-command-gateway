use tracing::{info, warn};

pub(crate) async fn send_response_event(
    tx: crate::qos::ReliableEventSender,
    resp: protocol::CommandResponse,
    command_name: &str,
) {
    let summary = ResponseSummary::from_response(&resp, command_name);
    summary.emit_structured_log(&resp);
    summary.emit_human_log(&resp);
    tx.send_event(resp, command_name).await;
}

struct ResponseSummary<'a> {
    command_name: &'a str,
    response_code: String,
    response_ok: bool,
    response_bytes: Option<usize>,
    chunk_mode: crate::log_view::ChunkMode,
    chunk_sizes: Vec<usize>,
}

impl<'a> ResponseSummary<'a> {
    fn from_response(resp: &protocol::CommandResponse, command_name: &'a str) -> Self {
        let response_bytes = protocol::encode_response(resp)
            .map(|value| value.len())
            .ok();
        let chunks = protocol::chunking::chunk_response(resp.clone());
        let chunk_mode = if chunks.len() > 1 {
            crate::log_view::ChunkMode::ResponseJson
        } else {
            crate::log_view::ChunkMode::Single
        };
        let chunk_sizes = chunks
            .iter()
            .filter_map(|chunk| match protocol::encode_response(chunk) {
                Ok(ser) => Some(ser.len()),
                Err(err) => {
                    warn!(
                        request_id = %resp.id,
                        cmd = %command_name,
                        error = %err,
                        "ble.response.encode_failed"
                    );
                    None
                }
            })
            .collect();

        Self {
            command_name,
            response_code: resp.code.clone(),
            response_ok: resp.ok,
            response_bytes,
            chunk_mode,
            chunk_sizes,
        }
    }

    fn emit_structured_log(&self, resp: &protocol::CommandResponse) {
        info!(
            request_id = %resp.id,
            cmd = %self.command_name,
            response_code = %self.response_code,
            response_ok = self.response_ok,
            phase = ?resp.phase,
            seq = resp.seq,
            final_flag = resp.final_flag,
            chunk_count = self.chunk_sizes.len(),
            chunk_mode = self.chunk_mode.as_str(),
            chunk_sizes = ?self.chunk_sizes,
            payload_limit = protocol::config::MAX_BLE_PAYLOAD_BYTES,
            response_bytes = self.response_bytes,
            max_chunk_bytes = self.max_chunk_bytes(),
            "ble.response.sent"
        );
    }

    fn emit_human_log(&self, resp: &protocol::CommandResponse) {
        crate::log_view::emit_block(&crate::log_view::response_block(
            &crate::log_view::ResponseLogView {
                request_id: &resp.id,
                command_name: self.command_name,
                response_code: &self.response_code,
                response_ok: self.response_ok,
                response_bytes: self.response_bytes,
                payload_limit: protocol::config::MAX_BLE_PAYLOAD_BYTES,
                chunk_mode: self.chunk_mode,
                chunk_sizes: &self.chunk_sizes,
            },
        ));
    }

    fn max_chunk_bytes(&self) -> usize {
        self.chunk_sizes.iter().copied().max().unwrap_or(0)
    }
}
