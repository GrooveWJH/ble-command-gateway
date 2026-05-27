use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod chunking;
pub mod requests;
pub mod responses;

pub const PROTOCOL_VERSION: &str = "YundroneBT-V2.1.0";

pub mod codes {
    pub const CODE_OK: &str = "OK";
    pub const CODE_ACCEPTED: &str = "ACCEPTED";
    pub const CODE_BAD_JSON: &str = "BAD_JSON";
    pub const CODE_BAD_REQUEST: &str = "BAD_REQUEST";
    pub const CODE_ACK_BAD_REQUEST: &str = "ACK_BAD_REQUEST";
    pub const CODE_UNKNOWN_COMMAND: &str = "UNKNOWN_COMMAND";
    pub const CODE_BUSY: &str = "BUSY";
    pub const CODE_IN_PROGRESS: &str = "IN_PROGRESS";
    pub const CODE_PARTIAL_SUCCESS: &str = "PARTIAL_SUCCESS";
    pub const CODE_PROTECTED_PROFILE: &str = "PROTECTED_PROFILE";
    pub const CODE_PROVISION_SUCCESS: &str = "PROVISION_SUCCESS";
    pub const CODE_PROVISION_FAIL: &str = "PROVISION_FAIL";
    pub const CODE_INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const CODE_REQUEST_EXPIRED: &str = "REQUEST_EXPIRED";
    pub const CODE_DELIVERY_TIMEOUT: &str = "DELIVERY_TIMEOUT";
    pub const CODE_TIMEOUT: &str = "TIMEOUT";
}

pub mod commands {
    pub const CMD_LINK_ACK: &str = "link.ack";
    pub const CMD_LINK_HEARTBEAT: &str = "link.heartbeat";
    pub const CMD_SYSTEM_STATUS: &str = "system.status";
    pub const CMD_SYSTEM_CAPABILITIES: &str = "system.capabilities";
    pub const CMD_WIFI_SCAN: &str = "wifi.scan";
    pub const CMD_WIFI_PROVISION: &str = "wifi.provision";
    pub const CMD_WIFI_PROFILES_LIST: &str = "wifi.profiles.list";
    pub const CMD_WIFI_PROFILES_DELETE: &str = "wifi.profiles.delete";
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponsePhase {
    Accepted,
    Progress,
    Result,
}

impl ResponsePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Progress => "progress",
            Self::Result => "result",
        }
    }
}

fn default_response_phase() -> ResponsePhase {
    ResponsePhase::Result
}

fn default_seq() -> u64 {
    1
}

fn default_final_flag() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    pub id: String,
    pub payload: requests::CommandPayload,
    pub v: String,
}

fn default_args() -> serde_json::Map<String, Value> {
    serde_json::Map::new()
}

fn default_version() -> String {
    PROTOCOL_VERSION.to_string()
}

impl CommandRequest {
    pub fn new(id: impl Into<String>, payload: requests::CommandPayload) -> Self {
        Self {
            id: id.into(),
            payload,
            v: default_version(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
    #[serde(default = "default_response_phase")]
    pub phase: ResponsePhase,
    #[serde(default = "default_seq")]
    pub seq: u64,
    #[serde(default = "default_final_flag", rename = "final")]
    pub final_flag: bool,
    pub ok: bool,
    pub code: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Map<String, Value>>,
    #[serde(default = "default_version")]
    pub v: String,
}

impl CommandResponse {
    pub fn ok(
        id: impl Into<String>,
        text: impl Into<String>,
        data: Option<serde_json::Map<String, Value>>,
    ) -> Self {
        Self::result(id, None, true, codes::CODE_OK, text, data)
    }

    pub fn result(
        id: impl Into<String>,
        cmd: Option<String>,
        ok: bool,
        code: impl Into<String>,
        text: impl Into<String>,
        data: Option<serde_json::Map<String, Value>>,
    ) -> Self {
        Self {
            id: id.into(),
            cmd,
            phase: ResponsePhase::Result,
            seq: 1,
            final_flag: true,
            ok,
            code: code.into(),
            text: text.into(),
            data,
            v: default_version(),
        }
    }

    pub fn error(id: impl Into<String>, code: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            cmd: None,
            phase: ResponsePhase::Result,
            seq: 1,
            final_flag: true,
            ok: false,
            code: code.into(),
            text: text.into(),
            data: None,
            v: default_version(),
        }
    }

    pub fn accepted(
        id: impl Into<String>,
        cmd: impl Into<String>,
        text: impl Into<String>,
        data: Option<serde_json::Map<String, Value>>,
    ) -> Self {
        Self {
            id: id.into(),
            cmd: Some(cmd.into()),
            phase: ResponsePhase::Accepted,
            seq: 1,
            final_flag: false,
            ok: true,
            code: codes::CODE_ACCEPTED.to_string(),
            text: text.into(),
            data,
            v: default_version(),
        }
    }

    pub fn progress(
        id: impl Into<String>,
        cmd: impl Into<String>,
        seq: u64,
        text: impl Into<String>,
        data: Option<serde_json::Map<String, Value>>,
    ) -> Self {
        Self {
            id: id.into(),
            cmd: Some(cmd.into()),
            phase: ResponsePhase::Progress,
            seq,
            final_flag: false,
            ok: true,
            code: codes::CODE_IN_PROGRESS.to_string(),
            text: text.into(),
            data,
            v: default_version(),
        }
    }

    pub fn decode_data<T: serde::de::DeserializeOwned>(&self) -> Result<T, ProtocolError> {
        let data = self
            .data
            .as_ref()
            .ok_or_else(|| ProtocolError::BadRequest("response data is missing".to_string()))?;
        responses::from_map(data)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("Bad JSON: {0}")]
    BadJson(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
}

pub fn parse_request(raw: &[u8]) -> Result<CommandRequest, ProtocolError> {
    let wire: WireCommandRequest =
        serde_json::from_slice(raw).map_err(|e| ProtocolError::BadJson(e.to_string()))?;
    if wire.v != PROTOCOL_VERSION {
        return Err(ProtocolError::BadRequest(format!(
            "unsupported protocol version: {}",
            wire.v
        )));
    }
    Ok(CommandRequest {
        id: wire.id,
        payload: requests::CommandPayload::from_wire(&wire.cmd, wire.args)?,
        v: wire.v,
    })
}

pub fn parse_error_response(raw: &[u8], err: &ProtocolError) -> Option<CommandResponse> {
    let value = serde_json::from_slice::<Value>(raw).ok()?;
    let object = value.as_object()?;
    let id = object.get("id")?.as_str()?.to_string();
    let cmd = object
        .get("cmd")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let (code, text) = match err {
        ProtocolError::BadRequest(message) if message.starts_with("unknown command:") => {
            (codes::CODE_UNKNOWN_COMMAND, message.clone())
        }
        ProtocolError::BadRequest(message) => (codes::CODE_BAD_REQUEST, message.clone()),
        ProtocolError::BadJson(message) => (codes::CODE_BAD_JSON, message.clone()),
    };
    Some(CommandResponse::result(id, cmd, false, code, text, None))
}

pub fn encode_request(req: &CommandRequest) -> Result<Vec<u8>, ProtocolError> {
    let wire = WireCommandRequest {
        id: req.id.clone(),
        cmd: req.payload.command_name().to_string(),
        args: req.payload.to_args_map(),
        v: req.v.clone(),
    };
    serde_json::to_vec(&wire).map_err(|e| ProtocolError::BadJson(e.to_string()))
}

pub fn parse_response(raw: &[u8]) -> Result<CommandResponse, ProtocolError> {
    serde_json::from_slice(raw).map_err(|e| ProtocolError::BadJson(e.to_string()))
}

pub fn encode_response(res: &CommandResponse) -> Result<Vec<u8>, ProtocolError> {
    serde_json::to_vec(res).map_err(|e| ProtocolError::BadJson(e.to_string()))
}

pub mod config {
    pub const MAX_BLE_PAYLOAD_BYTES: usize = 360;
    pub const DEFAULT_DEVICE_NAME: &str = "yundrone";
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkMeta {
    pub index: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WireCommandRequest {
    id: String,
    cmd: String,
    #[serde(default = "default_args")]
    args: serde_json::Map<String, Value>,
    #[serde(default = "default_version")]
    v: String,
}

#[cfg(test)]
mod tests;
