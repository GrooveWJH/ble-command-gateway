use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum CommandRunStatus {
    Succeeded(i32),
    Failed(i32),
    Error(String),
    TimedOut,
    InvalidInput(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CommandRunOutput {
    pub status: CommandRunStatus,
    pub stdout: String,
    pub stderr: String,
}

impl CommandRunOutput {
    pub fn preferred_text(&self) -> String {
        let mut text = if !self.stdout.is_empty() {
            self.stdout.clone()
        } else if !self.stderr.is_empty() {
            self.stderr.clone()
        } else {
            match self.status {
                CommandRunStatus::Succeeded(code) | CommandRunStatus::Failed(code) => {
                    format!("rc={code}")
                }
                CommandRunStatus::Error(ref message)
                | CommandRunStatus::InvalidInput(ref message) => message.clone(),
                CommandRunStatus::TimedOut => "command timed out".to_string(),
            }
        };

        if text.len() > 2000 {
            text = format!("{}...(truncated)", &text[..2000]);
        }
        text
    }
}

pub(super) async fn run_command_with_timeout(cmd: Vec<&str>, timeout_sec: f64) -> CommandRunOutput {
    if cmd.is_empty() {
        return CommandRunOutput {
            status: CommandRunStatus::InvalidInput("empty command".to_string()),
            stdout: String::new(),
            stderr: String::new(),
        };
    }

    let mut command = Command::new(cmd[0]);
    if cmd.len() > 1 {
        command.args(&cmd[1..]);
    }

    match timeout(Duration::from_secs_f64(timeout_sec), command.output()).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let status = if output.status.success() {
                CommandRunStatus::Succeeded(output.status.code().unwrap_or(0))
            } else {
                CommandRunStatus::Failed(output.status.code().unwrap_or(-1))
            };
            CommandRunOutput {
                status,
                stdout,
                stderr,
            }
        }
        Ok(Err(err)) => CommandRunOutput {
            status: CommandRunStatus::Error(format!("command error: {err}")),
            stdout: String::new(),
            stderr: String::new(),
        },
        Err(_) => CommandRunOutput {
            status: CommandRunStatus::TimedOut,
            stdout: String::new(),
            stderr: String::new(),
        },
    }
}
