use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod chunking;
pub mod requests;
pub mod responses;

pub const PROTOCOL_VERSION: &str = "YundroneBT-V1.0.0";

pub mod codes {
    pub const CODE_OK: &str = "OK";
    pub const CODE_BAD_JSON: &str = "BAD_JSON";
    pub const CODE_BAD_REQUEST: &str = "BAD_REQUEST";
    pub const CODE_UNKNOWN_COMMAND: &str = "UNKNOWN_COMMAND";
    pub const CODE_BUSY: &str = "BUSY";
    pub const CODE_IN_PROGRESS: &str = "IN_PROGRESS";
    pub const CODE_PROVISION_SUCCESS: &str = "PROVISION_SUCCESS";
    pub const CODE_PROVISION_FAIL: &str = "PROVISION_FAIL";
    pub const CODE_INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const CODE_TIMEOUT: &str = "TIMEOUT";
}

pub mod commands {
    pub const CMD_HELP: &str = "help";
    pub const CMD_PING: &str = "ping";
    pub const CMD_STATUS: &str = "status";
    pub const CMD_PROVISION: &str = "provision";
    pub const CMD_SHUTDOWN: &str = "shutdown";
    pub const CMD_SYS_WHOAMI: &str = "sys.whoami";
    pub const CMD_NET_IFCONFIG: &str = "net.ifconfig";
    pub const CMD_WIFI_SCAN: &str = "wifi.scan";
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
        Self {
            id: id.into(),
            ok: true,
            code: codes::CODE_OK.to_string(),
            text: text.into(),
            data,
            v: default_version(),
        }
    }

    pub fn error(id: impl Into<String>, code: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ok: false,
            code: code.into(),
            text: text.into(),
            data: None,
            v: default_version(),
        }
    }

    pub fn status(
        id: impl Into<String>,
        code: impl Into<String>,
        text: impl Into<String>,
        final_flag: bool,
        data: Option<serde_json::Map<String, Value>>,
    ) -> Self {
        let code_str = code.into();
        let is_ok = code_str != codes::CODE_PROVISION_FAIL;

        let mut body = data.unwrap_or_default();
        body.insert("final".to_string(), Value::Bool(final_flag));

        Self {
            id: id.into(),
            ok: is_ok,
            code: code_str,
            text: text.into(),
            data: Some(body),
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
    Ok(CommandRequest {
        id: wire.id,
        payload: requests::CommandPayload::from_wire(&wire.cmd, wire.args)?,
        v: wire.v,
    })
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
    pub const DEFAULT_DEVICE_NAME: &str = "Yundrone_UAV";
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
