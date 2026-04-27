use eframe::egui;

use super::model::{latest_feedback_for_slots, ActionFeedback, ActionPhase, ActionSlot, AppModel};
use super::theme::{failure_color, success_color, warning_color};
use crate::i18n::Lang;

pub(crate) const DEVICE_ACTION_SLOTS: [ActionSlot; 3] = [
    ActionSlot::DeviceScan,
    ActionSlot::Connect,
    ActionSlot::Disconnect,
];
pub(crate) const PROVISION_ACTION_SLOTS: [ActionSlot; 2] =
    [ActionSlot::WifiScan, ActionSlot::Provision];
pub(crate) const DIAGNOSTIC_ACTION_SLOTS: [ActionSlot; 3] =
    [ActionSlot::Status, ActionSlot::Ping, ActionSlot::Help];
pub(crate) const LOG_ACTION_SLOTS: [ActionSlot; 3] = [
    ActionSlot::RawSend,
    ActionSlot::LogsCopy,
    ActionSlot::LogsClear,
];

pub(crate) fn render_action_status(ui: &mut egui::Ui, model: &AppModel, slots: &[ActionSlot]) {
    let Some(feedback) = latest_feedback_for_slots(model, slots) else {
        return;
    };

    let (fill, stroke, text) = action_style_and_text(ui, model.lang, feedback);
    let frame = egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .inner_margin(egui::Margin::symmetric(8.0, 6.0))
        .rounding(egui::Rounding::same(6.0));

    ui.add_space(6.0);
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            if feedback.phase == ActionPhase::Running {
                ui.add(egui::Spinner::new().size(12.0));
            }
            ui.label(egui::RichText::new(text).size(12.0));
        });
    });
}

pub(crate) fn panel_is_refreshing(model: &AppModel, slots: &[ActionSlot]) -> bool {
    model
        .active_action
        .map(|slot| slots.contains(&slot))
        .unwrap_or(false)
}

pub(crate) fn render_refreshing_badge(ui: &mut egui::Ui, model: &AppModel, slots: &[ActionSlot]) {
    if panel_is_refreshing(model, slots) {
        let text = match model.lang {
            Lang::Zh => "正在刷新，旧结果仅供参考",
            Lang::En => "Refreshing; previous result shown",
        };
        ui.label(egui::RichText::new(text).color(warning_color(ui)));
    }
}

fn action_style_and_text(
    ui: &egui::Ui,
    lang: Lang,
    feedback: &ActionFeedback,
) -> (egui::Color32, egui::Color32, String) {
    match feedback.phase {
        ActionPhase::Running => (
            ui.visuals().widgets.inactive.bg_fill,
            warning_color(ui),
            running_text(lang, feedback.slot).to_string(),
        ),
        ActionPhase::Succeeded => (
            ui.visuals().faint_bg_color,
            success_color(ui),
            success_text(lang, feedback),
        ),
        ActionPhase::Failed => (
            ui.visuals().faint_bg_color,
            failure_color(ui),
            failure_text(lang, feedback),
        ),
        ActionPhase::Idle => (
            ui.visuals().faint_bg_color,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
            idle_text(lang).to_string(),
        ),
    }
}

fn running_text(lang: Lang, slot: ActionSlot) -> &'static str {
    match (lang, slot) {
        (Lang::Zh, ActionSlot::DeviceScan) => "正在搜索设备...",
        (Lang::En, ActionSlot::DeviceScan) => "Searching for devices...",
        (Lang::Zh, ActionSlot::Connect) => "正在连接设备...",
        (Lang::En, ActionSlot::Connect) => "Connecting to device...",
        (Lang::Zh, ActionSlot::Disconnect) => "正在断开连接...",
        (Lang::En, ActionSlot::Disconnect) => "Disconnecting...",
        (Lang::Zh, ActionSlot::WifiScan) => "正在扫描周边 Wi-Fi...",
        (Lang::En, ActionSlot::WifiScan) => "Scanning nearby Wi-Fi...",
        (Lang::Zh, ActionSlot::Provision) => "正在发送配网请求...",
        (Lang::En, ActionSlot::Provision) => "Sending provisioning request...",
        (Lang::Zh, ActionSlot::Status) => "正在抓取系统信息...",
        (Lang::En, ActionSlot::Status) => "Fetching system info...",
        (Lang::Zh, ActionSlot::Ping) => "正在执行连通性测试...",
        (Lang::En, ActionSlot::Ping) => "Running reachability test...",
        (Lang::Zh, ActionSlot::Help) => "正在请求远程支持信息...",
        (Lang::En, ActionSlot::Help) => "Requesting remote help...",
        (Lang::Zh, ActionSlot::RawSend) => "正在发送原始负载...",
        (Lang::En, ActionSlot::RawSend) => "Sending raw payload...",
        (Lang::Zh, ActionSlot::LogsCopy) => "正在复制日志...",
        (Lang::En, ActionSlot::LogsCopy) => "Copying logs...",
        (Lang::Zh, ActionSlot::LogsClear) => "正在清空日志...",
        (Lang::En, ActionSlot::LogsClear) => "Clearing logs...",
    }
}

fn success_text(lang: Lang, feedback: &ActionFeedback) -> String {
    let detail = feedback.detail.as_deref();
    match (lang, feedback.slot) {
        (Lang::Zh, ActionSlot::DeviceScan) if detail == Some("stopped") => {
            "扫描已手动停止。".into()
        }
        (Lang::En, ActionSlot::DeviceScan) if detail == Some("stopped") => {
            "Scan stopped by user.".into()
        }
        (Lang::Zh, ActionSlot::DeviceScan) => {
            format!("扫描完成，发现 {} 个候选设备。", detail.unwrap_or("0"))
        }
        (Lang::En, ActionSlot::DeviceScan) => format!(
            "Scan complete, found {} candidate device(s).",
            detail.unwrap_or("0")
        ),
        (Lang::Zh, ActionSlot::Connect) => format!("已连接到 {}。", detail.unwrap_or("设备")),
        (Lang::En, ActionSlot::Connect) => format!("Connected to {}.", detail.unwrap_or("device")),
        (Lang::Zh, ActionSlot::Disconnect) => format!("已断开 {}。", detail.unwrap_or("设备")),
        (Lang::En, ActionSlot::Disconnect) => {
            format!("Disconnected from {}.", detail.unwrap_or("device"))
        }
        (Lang::Zh, ActionSlot::WifiScan) => format!("已发现 {} 个网络。", detail.unwrap_or("0")),
        (Lang::En, ActionSlot::WifiScan) => {
            format!("Discovered {} network(s).", detail.unwrap_or("0"))
        }
        (Lang::Zh, ActionSlot::Provision) => "配网请求已完成。".into(),
        (Lang::En, ActionSlot::Provision) => "Provision request completed.".into(),
        (Lang::Zh, ActionSlot::Status) => "系统信息已更新。".into(),
        (Lang::En, ActionSlot::Status) => "System info updated.".into(),
        (Lang::Zh, ActionSlot::Ping) => "连通性测试已完成。".into(),
        (Lang::En, ActionSlot::Ping) => "Reachability test completed.".into(),
        (Lang::Zh, ActionSlot::Help) => "远程支持信息已更新。".into(),
        (Lang::En, ActionSlot::Help) => "Remote help updated.".into(),
        (Lang::Zh, ActionSlot::RawSend) => "原始负载已写入 BLE 特征。".into(),
        (Lang::En, ActionSlot::RawSend) => "Raw payload written to BLE characteristic.".into(),
        (Lang::Zh, ActionSlot::LogsCopy) => "日志已复制。".into(),
        (Lang::En, ActionSlot::LogsCopy) => "Logs copied.".into(),
        (Lang::Zh, ActionSlot::LogsClear) => "日志已清空。".into(),
        (Lang::En, ActionSlot::LogsClear) => "Logs cleared.".into(),
    }
}

fn failure_text(lang: Lang, feedback: &ActionFeedback) -> String {
    let error = feedback.error.as_deref().unwrap_or("unknown error");
    match lang {
        Lang::Zh => format!("操作失败: {error}"),
        Lang::En => format!("Action failed: {error}"),
    }
}

fn idle_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Zh => "空闲",
        Lang::En => "Idle",
    }
}
