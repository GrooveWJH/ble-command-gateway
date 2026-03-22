use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use anyhow::{Result, Context};
use serde_json::json;
use tracing::{info, debug, warn};

#[derive(Debug, Clone)]
pub struct SystemExecResult {
    pub ok: bool,
    pub text: String,
}

impl SystemExecResult {
    pub fn new(ok: bool, text: impl Into<String>) -> Self {
        Self { ok, text: text.into() }
    }
}

pub async fn run_named_command(name: &str, args: Option<serde_json::Map<String, serde_json::Value>>, timeout_sec: f64) -> SystemExecResult {
    let ifname = args.as_ref().and_then(|a| a.get("ifname")).and_then(|v| v.as_str());

    match name {
        protocol::commands::CMD_SYS_WHOAMI => run_effective_user(timeout_sec).await,
        protocol::commands::CMD_NET_IFCONFIG => run_ifconfig(ifname, timeout_sec).await,
        protocol::commands::CMD_WIFI_SCAN => run_wifi_scan(ifname).await,
        "provision" => run_wifi_provision(args).await,
        "shutdown" => run_command_with_timeout(vec!["shutdown", "-h", "now"], timeout_sec).await,
        "__status.hostname" => run_command_with_timeout(vec!["hostname"], timeout_sec).await,
        "__status.system" => run_command_with_timeout(vec!["uname", "-srm"], timeout_sec).await,
        "__status.user" => run_effective_user(timeout_sec).await,
        _ => SystemExecResult::new(false, format!("unsupported system command: {}", name)),
    }
}

async fn run_wifi_provision(args: Option<serde_json::Map<String, serde_json::Value>>) -> SystemExecResult {
    let args = match args {
        Some(a) => a,
        None => return SystemExecResult::new(false, "No arguments provided for provisioning"),
    };

    let ssid = args.get("ssid").and_then(|v| v.as_str()).unwrap_or("");
    let pwd = args.get("pwd").and_then(|v| v.as_str()).unwrap_or("");
    
    if ssid.is_empty() {
        return SystemExecResult::new(false, "SSID cannot be empty");
    }

    info!("Starting Wi-Fi provisioning to SSID: {}", ssid);

    let mut cmd = vec!["nmcli", "device", "wifi", "connect", ssid];
    if !pwd.is_empty() {
        cmd.push("password");
        cmd.push(pwd);
    }
    
    // Provisioning can take a while (authentication phase, DHCP)
    let res = run_command_with_timeout(cmd, 30.0).await;
    
    if res.ok {
        // Fetch new IP via generic hostname -I
        let ip_res = run_command_with_timeout(vec!["hostname", "-I"], 2.0).await;
        let ip_text = if ip_res.ok { ip_res.text.trim().to_string() } else { "Unknown IP".to_string() };
        
        let payload = json!({
            "status": "connected",
            "ssid": ssid,
            "ip": ip_text
        });
        SystemExecResult::new(true, payload.to_string())
    } else {
        SystemExecResult::new(false, format!("Provisioning failed: {}", res.text))
    }
}



async fn run_command_with_timeout(cmd: Vec<&str>, timeout_sec: f64) -> SystemExecResult {
    if cmd.is_empty() {
        return SystemExecResult::new(false, "empty command");
    }
    
    let mut command = Command::new(cmd[0]);
    if cmd.len() > 1 {
        command.args(&cmd[1..]);
    }
    
    let fut = command.output();
    match timeout(Duration::from_secs_f64(timeout_sec), fut).await {
        Ok(Ok(output)) => {
            let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let mut text = if !out.is_empty() { out } else if !err.is_empty() { err } else { format!("rc={}", output.status.code().unwrap_or(-1)) };
            if text.len() > 2000 {
                text = format!("{}...(truncated)", &text[..2000]);
            }
            SystemExecResult::new(output.status.success(), text)
        }
        Ok(Err(e)) => SystemExecResult::new(false, format!("command error: {}", e)),
        Err(_) => SystemExecResult::new(false, format!("system command timeout after {:.1}s", timeout_sec)),
    }
}

async fn run_effective_user(timeout_sec: f64) -> SystemExecResult {
    if let Ok(user) = std::env::var("SUDO_USER") {
        let u = user.trim();
        if !u.is_empty() {
            return SystemExecResult::new(true, u);
        }
    }
    run_command_with_timeout(vec!["whoami"], timeout_sec).await
}

async fn run_ifconfig(ifname: Option<&str>, timeout_sec: f64) -> SystemExecResult {
    let mut cmd = vec!["ifconfig"];
    if let Some(i) = ifname {
        cmd.push(i);
    }
    run_command_with_timeout(cmd, timeout_sec).await
}

async fn run_wifi_scan(ifname: Option<&str>) -> SystemExecResult {
    let mut cmd = vec!["nmcli", "device", "wifi", "rescan"];
    if let Some(i) = ifname {
        cmd.extend_from_slice(&["ifname", i]);
    }
    let rescan = run_command_with_timeout(cmd, 6.0).await;
    if !rescan.ok {
        return rescan;
    }
    
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    let mut list_cmd = vec![
        "nmcli", "-t", "-f", "IN-USE,BSSID,SSID,CHAN,RATE,SIGNAL,BARS,SECURITY", "device", "wifi", "list"
    ];
    if let Some(i) = ifname {
        list_cmd.extend_from_slice(&["ifname", i]);
    }
    
    let listed = run_command_with_timeout(list_cmd, 8.0).await;
    if !listed.ok {
        return listed;
    }
    
    // Simplistic parser for demonstration. In a full port, the logic to unescape nmcli goes here.
    let entries: Vec<serde_json::Value> = listed.text.lines().filter_map(|line| {
        let line = line.trim();
        if line.is_empty() { return None; }
        // Very basic split for nmcli terse format (doesn't handle escaped colons properly)
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 8 { return None; }
        let ssid = parts[2];
        if ssid.is_empty() { return None; }
        let signal: i32 = parts[5].parse().unwrap_or(0);
        
        Some(json!({
            "ssid": ssid,
            "chan": parts[3],
            "signal": signal,
        }))
    }).collect();

    let mut payload = json!({
        "ifname": ifname.unwrap_or(""),
        "count": entries.len(),
        "aps": entries,
    });
    
    SystemExecResult::new(true, payload.to_string())
}
