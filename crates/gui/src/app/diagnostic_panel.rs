use eframe::egui;
use protocol::requests::CommandPayload;

use super::action_ui::{render_action_status, render_refreshing_badge, DIAGNOSTIC_ACTION_SLOTS};
use super::model::ActionSlot;
use super::GatewayApp;

impl GatewayApp {
    pub(super) fn render_diagnostic_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.model.lang.t("tab_diag"));
        ui.separator();
        ui.add_space(8.0);
        render_action_status(ui, &self.model, &DIAGNOSTIC_ACTION_SLOTS);
        ui.add_space(8.0);

        ui.add_enabled_ui(self.model.is_connected, |ui| {
            let busy = self.model.active_action.is_some();
            if ui
                .add_enabled(!busy, egui::Button::new(self.model.lang.t("cmd_status")))
                .clicked()
            {
                self.send_command(ActionSlot::Status, CommandPayload::Status);
            }
            ui.add_space(5.0);
            if ui
                .add_enabled(!busy, egui::Button::new(self.model.lang.t("cmd_ping")))
                .clicked()
            {
                self.send_command(ActionSlot::Ping, CommandPayload::Ping);
            }
            ui.add_space(5.0);
            if ui
                .add_enabled(!busy, egui::Button::new(self.model.lang.t("cmd_help")))
                .clicked()
            {
                self.send_command(ActionSlot::Help, CommandPayload::Help);
            }
        });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(self.model.lang.t("diag_result_title")).strong());
            render_refreshing_badge(ui, &self.model, &DIAGNOSTIC_ACTION_SLOTS);
        });
        ui.add_space(6.0);

        result_frame().show(ui, |ui| {
            if let Some(result) = &self.model.diagnostic_result {
                let status_text = if result.ok {
                    egui::RichText::new("OK").color(egui::Color32::from_rgb(120, 220, 130))
                } else {
                    egui::RichText::new("FAIL").color(egui::Color32::from_rgb(255, 120, 120))
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&result.title).strong());
                    ui.label(format!("({})", result.code));
                    ui.label(status_text);
                });
                ui.add_space(4.0);
                for line in &result.lines {
                    ui.label(egui::RichText::new(line).monospace().size(12.0));
                }
            } else {
                ui.label(self.model.lang.t("diag_result_empty"));
            }
        });
    }
}

fn result_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(22, 25, 30))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 66, 78)))
        .inner_margin(egui::Margin::same(10.0))
        .rounding(egui::Rounding::same(6.0))
}
