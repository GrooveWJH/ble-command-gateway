use crate::{config::MAX_BLE_PAYLOAD_BYTES, ChunkMeta, CommandResponse, ProtocolError};
use std::collections::HashMap;

/// Helper to accumulate chunked CommandResponses.
#[derive(Default)]
pub struct ChunkAssembler {
    // Maps request_id -> (total_chunks, collected_text, final_data)
    sessions: HashMap<String, SessionState>,
}

struct SessionState {
    total: usize,
    received: usize,
    text_parts: Vec<String>,
    final_data: Option<serde_json::Map<String, serde_json::Value>>,
    original_code: String,
    original_ok: bool,
}

impl ChunkAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an incoming response. If it completes a chunked message, returns the reassembled response.
    pub fn add_chunk(
        &mut self,
        mut resp: CommandResponse,
    ) -> Result<Option<CommandResponse>, ProtocolError> {
        let chunk_meta = if let Some(ref mut data) = resp.data {
            if let Some(chunk_val) = data.remove("chunk") {
                serde_json::from_value::<ChunkMeta>(chunk_val).ok()
            } else {
                None
            }
        } else {
            None
        };

        if let Some(meta) = chunk_meta {
            // It's a chunked message
            let state = self
                .sessions
                .entry(resp.id.clone())
                .or_insert_with(|| SessionState {
                    total: meta.total,
                    received: 0,
                    text_parts: vec![String::new(); meta.total],
                    final_data: None,
                    original_code: resp.code.clone(),
                    original_ok: resp.ok,
                });

            if meta.index > 0 && meta.index <= meta.total {
                if state.text_parts[meta.index - 1].is_empty() && !resp.text.is_empty() {
                    state.received += 1;
                }
                state.text_parts[meta.index - 1] = resp.text;
            }

            // The final chunk usually carries the data payload (if any)
            if let Some(data) = resp.data {
                if !data.is_empty() {
                    state.final_data = Some(data);
                }
            }

            // Check if complete
            if state.received == state.total || (meta.index == meta.total && meta.total == 1) {
                // total == 1 can happen if it was forced to pack chunk meta
                let completed = self.sessions.remove(&resp.id).unwrap();
                let full_text = completed.text_parts.join("");

                return Ok(Some(CommandResponse {
                    id: resp.id,
                    ok: completed.original_ok,
                    code: completed.original_code,
                    text: full_text,
                    data: completed.final_data,
                    v: resp.v,
                }));
            }

            Ok(None)
        } else {
            // Not chunked, return directly
            Ok(Some(resp))
        }
    }
}

/// Splits a CommandResponse into multiple smaller CommandResponses that fit under MAX_BLE_PAYLOAD_BYTES.
pub fn chunk_response(resp: CommandResponse) -> Vec<CommandResponse> {
    let serialized = crate::encode_response(&resp).unwrap_or_default();
    if serialized.len() <= MAX_BLE_PAYLOAD_BYTES {
        return vec![resp];
    }

    // Simplistic chunking: we just split the text evenly.
    // In production, we'd do a binary search to perfectly fit the JSON structure, but a static chunk size of ~200 characters is safe for 360 bytes MTU.
    let text = resp.text;
    let chunk_size = 150;
    let mut parts: Vec<String> = text
        .chars()
        .collect::<Vec<_>>()
        .chunks(chunk_size)
        .map(|c| c.iter().collect())
        .collect();

    if parts.is_empty() {
        parts.push(String::new());
    }

    let total = parts.len();
    let mut results = Vec::new();

    for (i, part) in parts.into_iter().enumerate() {
        let index = i + 1;
        let mut chunk_data = if index == total {
            resp.data.clone().unwrap_or_default()
        } else {
            serde_json::Map::new()
        };

        chunk_data.insert(
            "chunk".to_string(),
            serde_json::json!({
                "index": index,
                "total": total
            }),
        );

        results.push(CommandResponse {
            id: resp.id.clone(),
            ok: resp.ok,
            code: resp.code.clone(),
            text: part,
            data: Some(chunk_data),
            v: resp.v.clone(),
        });
    }

    results
}
