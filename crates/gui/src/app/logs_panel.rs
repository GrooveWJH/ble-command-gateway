use eframe::egui;

use super::action_ui::{render_action_status, LOG_ACTION_SLOTS};
use super::model::ActionSlot;
use super::model::{clear_logs, export_logs};
use super::GatewayApp;

impl GatewayApp {
    pub(super) fn render_logs_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(self.model.lang.t("tab_logs")).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.model.lang.t("logs_copy")).clicked() {
                    ui.ctx().copy_text(export_logs(&self.model.logs));
                    self.record_local_success(ActionSlot::LogsCopy, None);
                }
                if ui.button(self.model.lang.t("logs_clear")).clicked() {
                    clear_logs(&mut self.model);
                    self.record_local_success(ActionSlot::LogsClear, None);
                }
            });
        });
        ui.separator();
        render_action_status(ui, &self.model, &LOG_ACTION_SLOTS);
        ui.add_space(8.0);

        let log_font = egui::FontId::new(11.0, egui::FontFamily::Monospace);
        let log_color = egui::Color32::from_gray(220);
        let frame = egui::Frame::none()
            .fill(egui::Color32::from_rgb(16, 19, 24))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 54, 64)))
            .inner_margin(egui::Margin::same(8.0))
            .rounding(egui::Rounding::same(6.0));

        frame.show(ui, |ui| {
            ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 2.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height() - 52.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.model.logs {
                        ui.label(
                            egui::RichText::new(line)
                                .font(log_font.clone())
                                .color(log_color),
                        );
                    }
                });
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(self.model.lang.t("raw_send"));
            ui.add(
                egui::TextEdit::singleline(&mut self.model.command_input)
                    .font(egui::TextStyle::Monospace),
            );
            let busy = self.model.active_action.is_some();
            if ui
                .add_enabled(!busy, egui::Button::new(self.model.lang.t("btn_send")))
                .clicked()
            {
                self.send_raw_payload(self.model.command_input.clone());
            }
        });
    }
}
