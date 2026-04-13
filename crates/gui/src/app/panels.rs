use eframe::egui;

use super::action_ui::{render_action_status, DEVICE_ACTION_SLOTS};
use super::model::{
    format_scan_candidate_label, header_badge_text, heartbeat_summary, ActionSlot, Tab,
};
use super::GatewayApp;
use crate::ble_worker::BtleCommand;

impl GatewayApp {
    pub(super) fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(header_badge_text());
                ui.heading(self.model.lang.t("app_title"));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(self.model.lang.t("lang_switch")).clicked() {
                        self.toggle_lang();
                    }

                    ui.add_space(15.0);
                    render_connection_status(ui, &self.model);
                });
            });
            ui.add_space(8.0);
        });
    }

    pub(super) fn render_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("control_panel")
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.group(|ui| {
                    ui.label(self.model.lang.t("device_prefix"));
                    ui.text_edit_singleline(&mut self.model.device_name);
                    ui.add_space(10.0);
                    render_action_status(ui, &self.model, &DEVICE_ACTION_SLOTS);
                    ui.add_space(8.0);

                    if self.model.is_scanning {
                        ui.horizontal(|ui| {
                            let _ = ui.add_enabled(
                                false,
                                egui::Button::new(self.model.lang.t("scan_running_btn")),
                            );
                            if ui.button(self.model.lang.t("scan_stop_btn")).clicked() {
                                let _ = self.tokio_tx.send(BtleCommand::StopScan);
                            }
                        });
                    } else {
                        ui.add_enabled_ui(
                            !self.model.is_connected
                                && !self.model.is_connecting
                                && self.model.active_action.is_none(),
                            |ui| {
                                if ui.button(self.model.lang.t("scan_btn")).clicked() {
                                    let _ = self.tokio_tx.send(BtleCommand::ScanCandidates {
                                        prefix: self.model.device_name.clone(),
                                        timeout_secs: 30,
                                    });
                                }
                            },
                        );
                    }

                    if let Some(name) = &self.model.connected_device_name {
                        ui.add_space(10.0);
                        ui.label(format!("{} {}", self.model.lang.t("selected_device"), name));
                        ui.small(heartbeat_summary(&self.model));
                        if self.model.is_connected
                            && ui
                                .add_enabled(
                                    self.model.active_action.is_none(),
                                    egui::Button::new(self.model.lang.t("disconnect_btn")),
                                )
                                .clicked()
                        {
                            let _ = self.tokio_tx.send(BtleCommand::Disconnect);
                        }
                    }

                    if !self.model.scan_candidates.is_empty() {
                        ui.add_space(10.0);
                        ui.label(self.model.lang.t("scan_results"));
                        ui.small(self.model.lang.t("scan_results_hint"));
                        ui.add_space(6.0);

                        for candidate in self.model.scan_candidates.clone() {
                            let is_connecting_candidate = self.model.is_connecting
                                && self.model.connected_device_name.as_deref()
                                    == Some(candidate.name.as_str());
                            let label = if is_connecting_candidate {
                                format!(
                                    "{} [{}]",
                                    format_scan_candidate_label(&candidate),
                                    self.model.lang.t("conn_connecting")
                                )
                            } else {
                                format_scan_candidate_label(&candidate)
                            };
                            let can_connect = !self.model.is_connecting
                                && matches!(
                                    self.model.active_action,
                                    None | Some(ActionSlot::DeviceScan)
                                );
                            ui.add_enabled_ui(can_connect, |ui| {
                                if ui.button(label).clicked() {
                                    let _ = self.tokio_tx.send(BtleCommand::ConnectToCandidate {
                                        name: candidate.name,
                                    });
                                }
                            });
                        }
                    }
                });

                ui.add_space(20.0);
                ui.vertical(|ui| {
                    ui.selectable_value(
                        &mut self.model.current_tab,
                        Tab::Provision,
                        self.model.lang.t("tab_provision"),
                    );
                    ui.selectable_value(
                        &mut self.model.current_tab,
                        Tab::Diagnostic,
                        self.model.lang.t("tab_diag"),
                    );
                    ui.selectable_value(
                        &mut self.model.current_tab,
                        Tab::Logs,
                        self.model.lang.t("tab_logs"),
                    );
                });
            });
    }

    pub(super) fn render_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| match self.model.current_tab {
            Tab::Provision => self.render_provision_tab(ui),
            Tab::Diagnostic => self.render_diagnostic_tab(ui),
            Tab::Logs => self.render_logs_tab(ui),
        });
    }
}

fn render_connection_status(ui: &mut egui::Ui, model: &super::model::AppModel) {
    let (label, color) = if model.is_connected && model.heartbeat_failures > 0 {
        (
            model.lang.t("conn_yes_warn"),
            egui::Color32::from_rgb(255, 180, 70),
        )
    } else if model.is_connected {
        (model.lang.t("conn_yes"), egui::Color32::GREEN)
    } else if model.is_connecting {
        (
            model.lang.t("conn_connecting"),
            egui::Color32::from_rgb(255, 180, 70),
        )
    } else if model.is_scanning {
        (model.lang.t("conn_wait"), egui::Color32::YELLOW)
    } else {
        (model.lang.t("conn_no"), egui::Color32::RED)
    };

    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(egui::RichText::new(label).color(color));
    });
}
