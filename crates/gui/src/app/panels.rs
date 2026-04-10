use eframe::egui;
use protocol::requests::CommandPayload;

use super::model::{clear_logs, export_logs, format_scan_candidate_label, header_badge_text, Tab};
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
                    let status = if self.model.is_connected {
                        egui::RichText::new(self.model.lang.t("conn_yes"))
                            .color(egui::Color32::GREEN)
                    } else if self.model.is_scanning {
                        egui::RichText::new(self.model.lang.t("conn_wait"))
                            .color(egui::Color32::YELLOW)
                    } else {
                        egui::RichText::new(self.model.lang.t("conn_no")).color(egui::Color32::RED)
                    };
                    ui.label(status);
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

                    ui.add_enabled_ui(!self.model.is_scanning && !self.model.is_connected, |ui| {
                        if ui.button(self.model.lang.t("scan_btn")).clicked() {
                            let _ = self.tokio_tx.send(BtleCommand::ScanCandidates {
                                prefix: self.model.device_name.clone(),
                                timeout_secs: 10,
                            });
                        }
                    });

                    if let Some(name) = &self.model.connected_device_name {
                        ui.add_space(10.0);
                        ui.label(format!("{} {}", self.model.lang.t("selected_device"), name));
                    }

                    if !self.model.scan_candidates.is_empty() {
                        ui.add_space(10.0);
                        ui.label(self.model.lang.t("scan_results"));
                        ui.small(self.model.lang.t("scan_results_hint"));
                        ui.add_space(6.0);

                        for candidate in self.model.scan_candidates.clone() {
                            let label = format_scan_candidate_label(&candidate);
                            if ui.button(label).clicked() {
                                let _ = self.tokio_tx.send(BtleCommand::ConnectToCandidate {
                                    name: candidate.name,
                                });
                            }
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

    fn render_provision_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.model.lang.t("tab_provision"));
        ui.separator();
        ui.add_space(10.0);

        ui.add_enabled_ui(self.model.is_connected, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.model.lang.t("ssid_label"));
                ui.text_edit_singleline(&mut self.model.ssid_input);
            });
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(self.model.lang.t("pwd_label"));
                ui.add(egui::TextEdit::singleline(&mut self.model.pwd_input).password(true));
            });

            ui.add_space(15.0);
            ui.horizontal(|ui| {
                if ui.button(self.model.lang.t("scan_ap_btn")).clicked() {
                    self.send_command(CommandPayload::WifiScan { ifname: None });
                }
                if ui.button(self.model.lang.t("prov_btn")).clicked() {
                    self.send_command(CommandPayload::Provision {
                        ssid: self.model.ssid_input.clone(),
                        pwd: if self.model.pwd_input.is_empty() {
                            None
                        } else {
                            Some(self.model.pwd_input.clone())
                        },
                    });
                }
            });

            ui.add_space(15.0);
            if !self.model.wifi_list.is_empty() {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("wifi_table").striped(true).show(ui, |ui| {
                        ui.heading(self.model.lang.t("col_ssid"));
                        ui.heading(self.model.lang.t("col_signal"));
                        ui.heading(self.model.lang.t("col_channel"));
                        ui.end_row();

                        for ap in &self.model.wifi_list {
                            let ssid = ap.ssid.as_str();
                            let signal = ap.signal;
                            let channel = ap.channel.as_str();

                            if ui
                                .selectable_label(self.model.ssid_input == ssid, ssid)
                                .clicked()
                            {
                                self.model.ssid_input = ssid.to_string();
                            }
                            ui.label(format!("{} dBm", signal));
                            ui.label(channel);
                            ui.end_row();
                        }
                    });
                });
            }
        });
    }

    fn render_diagnostic_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.model.lang.t("tab_diag"));
        ui.separator();
        ui.add_space(10.0);

        ui.add_enabled_ui(self.model.is_connected, |ui| {
            if ui.button(self.model.lang.t("cmd_whoami")).clicked() {
                self.send_command(CommandPayload::SysWhoAmI);
            }
            ui.add_space(5.0);
            if ui.button(self.model.lang.t("cmd_ping")).clicked() {
                self.send_command(CommandPayload::Ping);
            }
            ui.add_space(5.0);
            if ui.button(self.model.lang.t("cmd_help")).clicked() {
                self.send_command(CommandPayload::Help);
            }
            ui.add_space(5.0);
            if ui.button(self.model.lang.t("cmd_shutdown")).clicked() {
                self.send_command(CommandPayload::Shutdown);
            }
        });
    }

    fn render_logs_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(self.model.lang.t("tab_logs"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.model.lang.t("logs_copy")).clicked() {
                    ui.ctx().copy_text(export_logs(&self.model.logs));
                }
                if ui.button(self.model.lang.t("logs_clear")).clicked() {
                    clear_logs(&mut self.model);
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 40.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.model.logs {
                    ui.monospace(line);
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(self.model.lang.t("raw_send"));
            ui.add(
                egui::TextEdit::singleline(&mut self.model.command_input)
                    .font(egui::TextStyle::Monospace),
            );
            if ui.button(self.model.lang.t("btn_send")).clicked() {
                let _ = self.tokio_tx.send(BtleCommand::SendRaw {
                    payload: self.model.command_input.clone(),
                });
            }
        });
    }
}
