use serde_json::{Map, Value};

mod command_runner;
mod network;
mod system_commands;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub struct SystemExecResult {
    pub ok: bool,
    pub code: String,
    pub text: String,
    pub data: Option<Map<String, Value>>,
}

impl SystemExecResult {
    pub fn ok(text: impl Into<String>, data: Option<Map<String, Value>>) -> Self {
        Self {
            ok: true,
            code: protocol::codes::CODE_OK.to_string(),
            text: text.into(),
            data,
        }
    }

    pub fn with_code(
        ok: bool,
        code: impl Into<String>,
        text: impl Into<String>,
        data: Option<Map<String, Value>>,
    ) -> Self {
        Self {
            ok,
            code: code.into(),
            text: text.into(),
            data,
        }
    }

    pub fn error(code: impl Into<String>, text: impl Into<String>) -> Self {
        Self::with_code(false, code, text, None)
    }
}

pub async fn run_payload_command(
    payload: &protocol::requests::CommandPayload,
    timeout_sec: f64,
) -> SystemExecResult {
    match payload {
        protocol::requests::CommandPayload::Help => system_commands::run_help(),
        protocol::requests::CommandPayload::Ping => system_commands::run_ping(),
        protocol::requests::CommandPayload::Status => {
            system_commands::run_status(timeout_sec).await
        }
        protocol::requests::CommandPayload::SysWhoAmI => {
            system_commands::run_effective_user(timeout_sec).await
        }
        protocol::requests::CommandPayload::NetIfconfig { ifname } => {
            system_commands::run_ifconfig(ifname.as_deref(), timeout_sec).await
        }
        protocol::requests::CommandPayload::WifiScan { ifname } => {
            network::run_wifi_scan(ifname.as_deref()).await
        }
        protocol::requests::CommandPayload::Provision { ssid, pwd } => {
            network::run_wifi_provision(ssid, pwd.as_deref()).await
        }
        protocol::requests::CommandPayload::Shutdown => {
            run_system_command(vec!["shutdown", "-h", "now"], timeout_sec).await
        }
    }
}

async fn run_system_command(cmd: Vec<&str>, timeout_sec: f64) -> SystemExecResult {
    map_run_output(
        command_runner::run_command_with_timeout(cmd, timeout_sec).await,
        timeout_sec,
    )
}

fn map_run_output(output: command_runner::CommandRunOutput, timeout_sec: f64) -> SystemExecResult {
    match output.status {
        command_runner::CommandRunStatus::Succeeded(_) => {
            SystemExecResult::ok(output.preferred_text(), None)
        }
        command_runner::CommandRunStatus::Failed(_)
        | command_runner::CommandRunStatus::Error(_) => SystemExecResult::error(
            protocol::codes::CODE_INTERNAL_ERROR,
            output.preferred_text(),
        ),
        command_runner::CommandRunStatus::TimedOut => SystemExecResult::error(
            protocol::codes::CODE_TIMEOUT,
            format!("system command timeout after {:.1}s", timeout_sec),
        ),
        command_runner::CommandRunStatus::InvalidInput(message) => {
            SystemExecResult::error(protocol::codes::CODE_BAD_REQUEST, message)
        }
    }
}
