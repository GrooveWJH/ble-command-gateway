use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AckType {
    Chunk,
    Event,
}

impl AckType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Chunk => "chunk",
            Self::Event => "event",
        }
    }

    fn from_str(value: &str) -> Result<Self, crate::ProtocolError> {
        match value {
            "chunk" => Ok(Self::Chunk),
            "event" => Ok(Self::Event),
            _ => Err(crate::ProtocolError::BadRequest(
                "argument 'ack_type' must be 'chunk' or 'event'".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkAckArgs {
    pub ack_type: AckType,
    pub response_seq: u64,
    pub chunk_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPayload {
    LinkAck(LinkAckArgs),
    LinkHeartbeat,
    SystemStatus,
    SystemCapabilities,
    WifiScan { ifname: Option<String> },
    WifiProvision { ssid: String, pwd: Option<String> },
    WifiProfilesList,
    WifiProfilesDelete { uuids: Vec<String>, force: bool },
}

impl CommandPayload {
    pub fn command_name(&self) -> &'static str {
        match self {
            Self::LinkAck(_) => crate::commands::CMD_LINK_ACK,
            Self::LinkHeartbeat => crate::commands::CMD_LINK_HEARTBEAT,
            Self::SystemStatus => crate::commands::CMD_SYSTEM_STATUS,
            Self::SystemCapabilities => crate::commands::CMD_SYSTEM_CAPABILITIES,
            Self::WifiScan { .. } => crate::commands::CMD_WIFI_SCAN,
            Self::WifiProvision { .. } => crate::commands::CMD_WIFI_PROVISION,
            Self::WifiProfilesList => crate::commands::CMD_WIFI_PROFILES_LIST,
            Self::WifiProfilesDelete { .. } => crate::commands::CMD_WIFI_PROFILES_DELETE,
        }
    }

    pub fn to_args_map(&self) -> Map<String, Value> {
        let mut args = Map::new();
        match self {
            Self::LinkHeartbeat
            | Self::SystemStatus
            | Self::SystemCapabilities
            | Self::WifiProfilesList => {}
            Self::LinkAck(ack) => {
                args.insert(
                    "ack_type".to_string(),
                    Value::String(ack.ack_type.as_str().to_string()),
                );
                args.insert(
                    "response_seq".to_string(),
                    Value::Number(serde_json::Number::from(ack.response_seq)),
                );
                if let Some(chunk_index) = ack.chunk_index {
                    args.insert(
                        "chunk_index".to_string(),
                        Value::Number(serde_json::Number::from(chunk_index)),
                    );
                }
            }
            Self::WifiScan { ifname } => {
                if let Some(ifname) = ifname {
                    args.insert("ifname".to_string(), Value::String(ifname.clone()));
                }
            }
            Self::WifiProvision { ssid, pwd } => {
                args.insert("ssid".to_string(), Value::String(ssid.clone()));
                if let Some(pwd) = pwd {
                    args.insert("pwd".to_string(), Value::String(pwd.clone()));
                }
            }
            Self::WifiProfilesDelete { uuids, force } => {
                args.insert(
                    "uuids".to_string(),
                    Value::Array(uuids.iter().cloned().map(Value::String).collect()),
                );
                if *force {
                    args.insert("force".to_string(), Value::Bool(true));
                }
            }
        }
        args
    }

    pub fn from_wire(cmd: &str, args: Map<String, Value>) -> Result<Self, crate::ProtocolError> {
        match cmd {
            crate::commands::CMD_LINK_ACK => Ok(Self::LinkAck(parse_link_ack_args(&args)?)),
            crate::commands::CMD_LINK_HEARTBEAT => {
                expect_empty_args(cmd, &args).map(|_| Self::LinkHeartbeat)
            }
            crate::commands::CMD_SYSTEM_STATUS => {
                expect_empty_args(cmd, &args).map(|_| Self::SystemStatus)
            }
            crate::commands::CMD_SYSTEM_CAPABILITIES => {
                expect_empty_args(cmd, &args).map(|_| Self::SystemCapabilities)
            }
            crate::commands::CMD_WIFI_SCAN => Ok(Self::WifiScan {
                ifname: optional_string_arg(&args, "ifname")?,
            }),
            crate::commands::CMD_WIFI_PROVISION => Ok(Self::WifiProvision {
                ssid: required_string_arg(&args, "ssid")?,
                pwd: optional_string_arg(&args, "pwd")?,
            }),
            crate::commands::CMD_WIFI_PROFILES_LIST => {
                expect_empty_args(cmd, &args).map(|_| Self::WifiProfilesList)
            }
            crate::commands::CMD_WIFI_PROFILES_DELETE => Ok(Self::WifiProfilesDelete {
                uuids: required_string_array_arg(&args, "uuids")?,
                force: optional_bool_arg(&args, "force")?.unwrap_or(false),
            }),
            _ => Err(crate::ProtocolError::BadRequest(format!(
                "unknown command: {cmd}"
            ))),
        }
    }
}

fn parse_link_ack_args(args: &Map<String, Value>) -> Result<LinkAckArgs, crate::ProtocolError> {
    let ack_type = AckType::from_str(&required_string_arg(args, "ack_type")?)?;
    let response_seq = required_u64_arg(args, "response_seq")?;
    let chunk_index = optional_usize_arg(args, "chunk_index")?;
    if matches!(ack_type, AckType::Chunk) {
        match chunk_index {
            Some(value) if value > 0 => {}
            Some(_) => {
                return Err(crate::ProtocolError::BadRequest(
                    "argument 'chunk_index' must be greater than 0".to_string(),
                ));
            }
            None => {
                return Err(crate::ProtocolError::BadRequest(
                    "argument 'chunk_index' is required for chunk ACK".to_string(),
                ));
            }
        }
    }
    Ok(LinkAckArgs {
        ack_type,
        response_seq,
        chunk_index,
    })
}

fn optional_bool_arg(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<bool>, crate::ProtocolError> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(crate::ProtocolError::BadRequest(format!(
            "argument '{key}' must be a boolean"
        ))),
    }
}

fn optional_usize_arg(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<usize>, crate::ProtocolError> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::Number(value)) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| {
                crate::ProtocolError::BadRequest(format!(
                    "argument '{key}' must be a positive integer"
                ))
            }),
        Some(_) => Err(crate::ProtocolError::BadRequest(format!(
            "argument '{key}' must be an integer"
        ))),
    }
}

fn required_u64_arg(args: &Map<String, Value>, key: &str) -> Result<u64, crate::ProtocolError> {
    match args.get(key) {
        Some(Value::Number(value)) => value.as_u64().ok_or_else(|| {
            crate::ProtocolError::BadRequest(format!("argument '{key}' must be an integer"))
        }),
        Some(_) => Err(crate::ProtocolError::BadRequest(format!(
            "argument '{key}' must be an integer"
        ))),
        None => Err(crate::ProtocolError::BadRequest(format!(
            "missing required argument '{key}'"
        ))),
    }
}

fn expect_empty_args(cmd: &str, args: &Map<String, Value>) -> Result<(), crate::ProtocolError> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(crate::ProtocolError::BadRequest(format!(
            "command '{cmd}' does not accept args"
        )))
    }
}

fn optional_string_arg(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, crate::ProtocolError> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(crate::ProtocolError::BadRequest(format!(
            "argument '{key}' must be a string"
        ))),
    }
}

fn required_string_arg(
    args: &Map<String, Value>,
    key: &str,
) -> Result<String, crate::ProtocolError> {
    optional_string_arg(args, key)?.ok_or_else(|| {
        crate::ProtocolError::BadRequest(format!("missing required argument '{key}'"))
    })
}

fn required_string_array_arg(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, crate::ProtocolError> {
    match args.get(key) {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                Value::String(value) => Ok(value.clone()),
                _ => Err(crate::ProtocolError::BadRequest(format!(
                    "argument '{key}' must be an array of strings"
                ))),
            })
            .collect(),
        Some(_) => Err(crate::ProtocolError::BadRequest(format!(
            "argument '{key}' must be an array of strings"
        ))),
        None => Err(crate::ProtocolError::BadRequest(format!(
            "missing required argument '{key}'"
        ))),
    }
}
