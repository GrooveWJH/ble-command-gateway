use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPayload {
    Help,
    Ping,
    Status,
    SysWhoAmI,
    NetIfconfig { ifname: Option<String> },
    WifiScan { ifname: Option<String> },
    Provision { ssid: String, pwd: Option<String> },
    Shutdown,
}

impl CommandPayload {
    pub fn command_name(&self) -> &'static str {
        match self {
            Self::Help => crate::commands::CMD_HELP,
            Self::Ping => crate::commands::CMD_PING,
            Self::Status => crate::commands::CMD_STATUS,
            Self::SysWhoAmI => crate::commands::CMD_SYS_WHOAMI,
            Self::NetIfconfig { .. } => crate::commands::CMD_NET_IFCONFIG,
            Self::WifiScan { .. } => crate::commands::CMD_WIFI_SCAN,
            Self::Provision { .. } => crate::commands::CMD_PROVISION,
            Self::Shutdown => crate::commands::CMD_SHUTDOWN,
        }
    }

    pub fn to_args_map(&self) -> Map<String, Value> {
        let mut args = Map::new();
        match self {
            Self::Help | Self::Ping | Self::Status | Self::SysWhoAmI | Self::Shutdown => {}
            Self::NetIfconfig { ifname } | Self::WifiScan { ifname } => {
                if let Some(ifname) = ifname {
                    args.insert("ifname".to_string(), Value::String(ifname.clone()));
                }
            }
            Self::Provision { ssid, pwd } => {
                args.insert("ssid".to_string(), Value::String(ssid.clone()));
                if let Some(pwd) = pwd {
                    args.insert("pwd".to_string(), Value::String(pwd.clone()));
                }
            }
        }
        args
    }

    pub fn from_wire(cmd: &str, args: Map<String, Value>) -> Result<Self, crate::ProtocolError> {
        match cmd {
            crate::commands::CMD_HELP => expect_empty_args(cmd, &args).map(|_| Self::Help),
            crate::commands::CMD_PING => expect_empty_args(cmd, &args).map(|_| Self::Ping),
            crate::commands::CMD_STATUS => expect_empty_args(cmd, &args).map(|_| Self::Status),
            crate::commands::CMD_SYS_WHOAMI => {
                expect_empty_args(cmd, &args).map(|_| Self::SysWhoAmI)
            }
            crate::commands::CMD_SHUTDOWN => expect_empty_args(cmd, &args).map(|_| Self::Shutdown),
            crate::commands::CMD_NET_IFCONFIG => Ok(Self::NetIfconfig {
                ifname: optional_string_arg(&args, "ifname")?,
            }),
            crate::commands::CMD_WIFI_SCAN => Ok(Self::WifiScan {
                ifname: optional_string_arg(&args, "ifname")?,
            }),
            crate::commands::CMD_PROVISION => Ok(Self::Provision {
                ssid: required_string_arg(&args, "ssid")?,
                pwd: optional_string_arg(&args, "pwd")?,
            }),
            _ => Err(crate::ProtocolError::BadRequest(format!(
                "unknown command: {cmd}"
            ))),
        }
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
