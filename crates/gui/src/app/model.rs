use client::ScanCandidateInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::i18n::Lang;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ActionSlot {
    DeviceScan,
    Connect,
    Disconnect,
    WifiScan,
    Provision,
    Status,
    Ping,
    Help,
    RawSend,
    LogsCopy,
    LogsClear,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionPhase {
    Idle,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionFeedback {
    pub slot: ActionSlot,
    pub phase: ActionPhase,
    pub detail: Option<String>,
    pub error: Option<String>,
    pub request_id: Option<String>,
    sequence: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DisconnectReason {
    Manual,
    HeartbeatFailed,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

impl ThemePreference {
    pub fn to_egui(self) -> eframe::egui::ThemePreference {
        match self {
            Self::Light => eframe::egui::ThemePreference::Light,
            Self::Dark => eframe::egui::ThemePreference::Dark,
            Self::System => eframe::egui::ThemePreference::System,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    Log(String),
    ActionStarted {
        slot: ActionSlot,
        request_id: Option<String>,
    },
    ActionSucceeded {
        slot: ActionSlot,
        request_id: Option<String>,
        detail: Option<String>,
    },
    ActionFailed {
        slot: ActionSlot,
        request_id: Option<String>,
        error: String,
    },
    ScanStarted,
    ScanCandidateDiscovered(ScanCandidateInfo),
    ScanFinished,
    ScanStopped,
    ConnectingToCandidate(String),
    ConnectedDeviceSelected(String),
    HeartbeatOk {
        at: String,
    },
    HeartbeatMissed(u8),
    Disconnected {
        reason: DisconnectReason,
    },
    WifiScanLoaded(Vec<protocol::responses::WifiNetwork>),
    DiagnosticResult(DiagnosticResultCard),
    ProvisionResult(ProvisionResultCard),
    CommandCompleted(CommandResultSummary),
    ConnectionFailed(String),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiagnosticResultCard {
    pub title: String,
    pub ok: bool,
    pub code: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProvisionResultCard {
    pub ok: bool,
    pub code: String,
    pub status: String,
    pub ssid: String,
    pub ip: Option<String>,
    pub text: String,
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
    pub theme_preference: ThemePreference,
    pub current_tab: Tab,
    pub device_name: String,
    pub connected_device_name: Option<String>,
    pub logs: Vec<String>,
    pub is_scanning: bool,
    pub is_connecting: bool,
    pub is_connected: bool,
    pub active_action: Option<ActionSlot>,
    pub action_feedback: HashMap<ActionSlot, ActionFeedback>,
    pub heartbeat_ok: bool,
    pub heartbeat_failures: u8,
    pub last_heartbeat_at: Option<String>,
    pub scan_candidates: Vec<ScanCandidateInfo>,
    pub ssid_input: String,
    pub pwd_input: String,
    pub wifi_list: Vec<protocol::responses::WifiNetwork>,
    pub diagnostic_result: Option<DiagnosticResultCard>,
    pub provision_result: Option<ProvisionResultCard>,
    pub command_input: String,
    pub(crate) next_feedback_sequence: u64,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            lang: Lang::Zh,
            theme_preference: ThemePreference::System,
            current_tab: Tab::Provision,
            device_name: "Yundrone_UAV".to_string(),
            connected_device_name: None,
            logs: vec!["[SYS] Init GUI engine...".into()],
            is_scanning: false,
            is_connecting: false,
            is_connected: false,
            active_action: None,
            action_feedback: HashMap::new(),
            heartbeat_ok: false,
            heartbeat_failures: 0,
            last_heartbeat_at: None,
            scan_candidates: vec![],
            ssid_input: String::new(),
            pwd_input: String::new(),
            wifi_list: vec![],
            diagnostic_result: None,
            provision_result: None,
            command_input: String::new(),
            next_feedback_sequence: 0,
        }
    }
}

impl AppModel {
    pub fn record_action_feedback(
        &mut self,
        slot: ActionSlot,
        phase: ActionPhase,
        request_id: Option<String>,
        detail: Option<String>,
        error: Option<String>,
    ) {
        self.next_feedback_sequence += 1;
        self.action_feedback.insert(
            slot,
            ActionFeedback {
                slot,
                phase,
                detail,
                error,
                request_id,
                sequence: self.next_feedback_sequence,
            },
        );
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

pub fn heartbeat_summary(model: &AppModel) -> String {
    if !model.is_connected {
        return model.lang.t("heartbeat_idle").to_string();
    }
    if model.heartbeat_failures > 0 {
        return format!(
            "{} ({}/{})",
            model.lang.t("heartbeat_missed"),
            model.heartbeat_failures,
            3
        );
    }
    match model.last_heartbeat_at.as_deref() {
        Some(at) => format!("{} {}", model.lang.t("heartbeat_last_ok"), at),
        None => model.lang.t("heartbeat_starting").to_string(),
    }
}

pub fn latest_feedback_for_slots<'a>(
    model: &'a AppModel,
    slots: &[ActionSlot],
) -> Option<&'a ActionFeedback> {
    slots
        .iter()
        .filter_map(|slot| model.action_feedback.get(slot))
        .max_by_key(|feedback| feedback.sequence)
}
