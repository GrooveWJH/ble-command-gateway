use eframe::egui;
use protocol::requests::CommandPayload;

use super::action_ui::{render_action_status, render_refreshing_badge, DIAGNOSTIC_ACTION_SLOTS};
use super::model::ActionSlot;
use super::theme::{failure_color, panel_frame, success_color};
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

        panel_frame(ui).show(ui, |ui| {
            if let Some(result) = &self.model.diagnostic_result {
                let status_text = if result.ok {
                    egui::RichText::new("OK").color(success_color(ui))
                } else {
                    egui::RichText::new("FAIL").color(failure_color(ui))
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
