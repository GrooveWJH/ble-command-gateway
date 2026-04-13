use crate::{config::MAX_BLE_PAYLOAD_BYTES, ChunkMeta, CommandResponse, ProtocolError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default)]
pub struct ChunkAssembler {
    sessions: HashMap<String, SessionState>,
}

struct SessionState {
    total: usize,
    received: usize,
    parts: Vec<String>,
    legacy_final_data: Option<serde_json::Map<String, serde_json::Value>>,
    legacy_original_code: String,
    legacy_original_ok: bool,
    response_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponseJsonChunkMeta {
    mode: String,
    index: usize,
    total: usize,
    payload: String,
}

impl ChunkAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_chunk(
        &mut self,
        mut resp: CommandResponse,
    ) -> Result<Option<CommandResponse>, ProtocolError> {
        if let Some(chunk) = parse_response_json_chunk(&mut resp.data)? {
            return self.add_response_json_chunk(resp.id, chunk);
        }
        if let Some(meta) = parse_legacy_chunk_meta(&mut resp.data)? {
            return self.add_legacy_chunk(resp, meta);
        }
        Ok(Some(resp))
    }

    fn add_response_json_chunk(
        &mut self,
        response_id: String,
        chunk: ResponseJsonChunkMeta,
    ) -> Result<Option<CommandResponse>, ProtocolError> {
        let state = self
            .sessions
            .entry(response_id.clone())
            .or_insert_with(|| SessionState {
                total: chunk.total,
                received: 0,
                parts: vec![String::new(); chunk.total],
                legacy_final_data: None,
                legacy_original_code: String::new(),
                legacy_original_ok: true,
                response_version: crate::PROTOCOL_VERSION.to_string(),
            });

        if chunk.index > 0 && chunk.index <= chunk.total {
            if state.parts[chunk.index - 1].is_empty() && !chunk.payload.is_empty() {
                state.received += 1;
            }
            state.parts[chunk.index - 1] = chunk.payload;
        }

        if state.received == state.total {
            let completed = self.sessions.remove(&response_id).unwrap();
            let raw = completed.parts.join("");
            return crate::parse_response(raw.as_bytes()).map(Some);
        }

        Ok(None)
    }

    fn add_legacy_chunk(
        &mut self,
        resp: CommandResponse,
        meta: ChunkMeta,
    ) -> Result<Option<CommandResponse>, ProtocolError> {
        let response_id = resp.id.clone();
        let response_text = resp.text.clone();
        let response_data = resp.data.clone();
        let state = self
            .sessions
            .entry(response_id.clone())
            .or_insert_with(|| SessionState {
                total: meta.total,
                received: 0,
                parts: vec![String::new(); meta.total],
                legacy_final_data: None,
                legacy_original_code: resp.code.clone(),
                legacy_original_ok: resp.ok,
                response_version: resp.v.clone(),
            });

        if meta.index > 0 && meta.index <= meta.total {
            if state.parts[meta.index - 1].is_empty() && !response_text.is_empty() {
                state.received += 1;
            }
            state.parts[meta.index - 1] = response_text;
        }

        if let Some(data) = response_data {
            if !data.is_empty() {
                state.legacy_final_data = Some(data);
            }
        }

        if state.received == state.total || (meta.index == meta.total && meta.total == 1) {
            let completed = self.sessions.remove(&response_id).unwrap();
            return Ok(Some(CommandResponse {
                id: response_id,
                ok: completed.legacy_original_ok,
                code: completed.legacy_original_code,
                text: completed.parts.join(""),
                data: completed.legacy_final_data,
                v: completed.response_version,
            }));
        }

        Ok(None)
    }
}

pub fn chunk_response(resp: CommandResponse) -> Vec<CommandResponse> {
    let serialized = crate::encode_response(&resp).unwrap_or_default();
    if serialized.len() <= MAX_BLE_PAYLOAD_BYTES {
        return vec![resp];
    }
    chunk_serialized_response(resp, String::from_utf8(serialized).unwrap_or_default())
}

fn chunk_serialized_response(resp: CommandResponse, raw_response: String) -> Vec<CommandResponse> {
    let chars: Vec<char> = raw_response.chars().collect();
    let mut payloads = Vec::new();
    let mut start = 0usize;

    while start < chars.len() {
        let payload = next_payload_fragment(&resp, &chars, start);
        start += payload.chars().count();
        payloads.push(payload);
    }

    let total = payloads.len();
    payloads
        .into_iter()
        .enumerate()
        .map(|(index, payload)| build_response_json_chunk(&resp, payload, index + 1, total))
        .collect()
}

fn next_payload_fragment(resp: &CommandResponse, chars: &[char], start: usize) -> String {
    let mut low = 1usize;
    let mut high = chars.len() - start;
    let mut best = 1usize;

    while low <= high {
        let mid = (low + high) / 2;
        let payload: String = chars[start..start + mid].iter().collect();
        if chunk_fits_limit(resp, &payload) {
            best = mid;
            low = mid + 1;
        } else {
            high = mid.saturating_sub(1);
        }
    }

    chars[start..start + best].iter().collect()
}

fn chunk_fits_limit(resp: &CommandResponse, payload: &str) -> bool {
    let chunk = build_response_json_chunk(resp, payload.to_string(), 1, 1);
    crate::encode_response(&chunk)
        .map(|encoded| encoded.len() <= MAX_BLE_PAYLOAD_BYTES)
        .unwrap_or(false)
}

fn build_response_json_chunk(
    resp: &CommandResponse,
    payload: String,
    index: usize,
    total: usize,
) -> CommandResponse {
    let mut data = serde_json::Map::new();
    data.insert(
        "chunk".to_string(),
        serde_json::to_value(ResponseJsonChunkMeta {
            mode: "response_json".to_string(),
            index,
            total,
            payload,
        })
        .expect("chunk metadata should serialize"),
    );

    CommandResponse {
        id: resp.id.clone(),
        ok: resp.ok,
        code: resp.code.clone(),
        text: String::new(),
        data: Some(data),
        v: resp.v.clone(),
    }
}

fn parse_response_json_chunk(
    data: &mut Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<Option<ResponseJsonChunkMeta>, ProtocolError> {
    let Some(map) = data.as_mut() else {
        return Ok(None);
    };
    let Some(chunk_val) = map.remove("chunk") else {
        return Ok(None);
    };
    match serde_json::from_value::<ResponseJsonChunkMeta>(chunk_val.clone()) {
        Ok(chunk) if chunk.mode == "response_json" => {
            if map.is_empty() {
                *data = None;
            }
            Ok(Some(chunk))
        }
        Ok(_) => Err(ProtocolError::BadRequest(
            "unsupported chunk mode".to_string(),
        )),
        Err(_) => {
            map.insert("chunk".to_string(), chunk_val);
            Ok(None)
        }
    }
}

fn parse_legacy_chunk_meta(
    data: &mut Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<Option<ChunkMeta>, ProtocolError> {
    let Some(map) = data.as_mut() else {
        return Ok(None);
    };
    let Some(chunk_val) = map.remove("chunk") else {
        return Ok(None);
    };
    let meta = serde_json::from_value::<ChunkMeta>(chunk_val)
        .map_err(|err| ProtocolError::BadJson(err.to_string()))?;
    if map.is_empty() {
        *data = None;
    }
    Ok(Some(meta))
}
