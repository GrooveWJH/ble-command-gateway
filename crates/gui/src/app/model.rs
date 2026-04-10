use client::ScanCandidateInfo;

use crate::i18n::Lang;

#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    Log(String),
    ScanStarted,
    ScanResults(Vec<ScanCandidateInfo>),
    ConnectedDeviceSelected(String),
    WifiScanLoaded(Vec<protocol::responses::WifiNetwork>),
    CommandCompleted(CommandResultSummary),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommandResultSummary {
    pub request_id: String,
    pub code: String,
    pub text: String,
    pub ok: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tab {
    Provision,
    Diagnostic,
    Logs,
}

pub struct AppModel {
    pub lang: Lang,
    pub current_tab: Tab,
    pub device_name: String,
    pub connected_device_name: Option<String>,
    pub logs: Vec<String>,
    pub is_scanning: bool,
    pub is_connected: bool,
    pub scan_candidates: Vec<ScanCandidateInfo>,
    pub ssid_input: String,
    pub pwd_input: String,
    pub wifi_list: Vec<protocol::responses::WifiNetwork>,
    pub command_input: String,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            lang: Lang::Zh,
            current_tab: Tab::Provision,
            device_name: "Yundrone_UAV".to_string(),
            connected_device_name: None,
            logs: vec!["[SYS] Init GUI engine...".into()],
            is_scanning: false,
            is_connected: false,
            scan_candidates: vec![],
            ssid_input: String::new(),
            pwd_input: String::new(),
            wifi_list: vec![],
            command_input: String::new(),
        }
    }
}

pub fn format_scan_candidate_label(candidate: &ScanCandidateInfo) -> String {
    let signal = candidate
        .rssi
        .map(|value| format!("{value} dBm"))
        .unwrap_or_else(|| "RSSI unknown".to_string());
    format!("{} ({signal})", candidate.name)
}

pub fn header_badge_text() -> &'static str {
    "BLE"
}

pub fn export_logs(logs: &[String]) -> String {
    logs.join("\n")
}

pub fn clear_logs(model: &mut AppModel) {
    model.logs.clear();
}
