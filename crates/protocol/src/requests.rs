use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPayload {
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
