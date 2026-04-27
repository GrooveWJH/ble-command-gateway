use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ProtocolError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HelpResponseData {
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PingResponseData {
    pub pong: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusInterfaceKind {
    Wifi,
    Ethernet,
    Other,
}

impl StatusInterfaceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wifi => "wifi",
            Self::Ethernet => "ethernet",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusInterfaceIpv4 {
    pub ifname: String,
    pub kind: StatusInterfaceKind,
    pub ipv4: String,
}

impl StatusInterfaceIpv4 {
    pub fn summary_line(&self) -> String {
        format!("{} [{}] -> {}", self.ifname, self.kind.as_str(), self.ipv4)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusResponseData {
    pub hostname: String,
    pub system: String,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<StatusInterfaceIpv4>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhoAmIResponseData {
    pub user: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub channel: String,
    pub signal: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WifiScanResponseData {
    pub ifname: Option<String>,
    pub count: u64,
    pub networks: Vec<WifiNetwork>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionState {
    Connected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvisionResponseData {
    pub status: ProvisionState,
    pub ssid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
}

pub fn to_map<T: Serialize>(value: &T) -> Result<Map<String, Value>, ProtocolError> {
    match serde_json::to_value(value).map_err(|err| ProtocolError::BadJson(err.to_string()))? {
        Value::Object(map) => Ok(map),
        other => Err(ProtocolError::BadRequest(format!(
            "expected object payload, got {other}"
        ))),
    }
}

pub fn from_map<T: DeserializeOwned>(map: &Map<String, Value>) -> Result<T, ProtocolError> {
    serde_json::from_value(Value::Object(map.clone()))
        .map_err(|err| ProtocolError::BadRequest(err.to_string()))
}
