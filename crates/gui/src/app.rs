use eframe::egui;
use std::sync::mpsc::{Receiver, Sender};

use crate::i18n::Lang;
use crate::ble_worker::{AppEvent, BtleCommand};

#[derive(PartialEq)]
pub enum Tab {
    Provision,
    Diagnostic,
    Logs,
}

pub struct GatewayApp {
    _ui_tx: Sender<AppEvent>,
    ui_rx: Receiver<AppEvent>,
    tokio_tx: tokio::sync::mpsc::UnboundedSender<BtleCommand>,

    lang: Lang,
    current_tab: Tab,

    device_name: String,
    logs: Vec<String>,
    is_scanning: bool,
    is_connected: bool,
    
    // Provisioning states
    ssid_input: String,
    pwd_input: String,
    wifi_list: Vec<serde_json::Value>,

    // Custom commands state
    command_input: String,
}

impl GatewayApp {
    pub fn new(
        ui_tx: Sender<AppEvent>,
        ui_rx: Receiver<AppEvent>,
        tokio_tx: tokio::sync::mpsc::UnboundedSender<BtleCommand>
    ) -> Self {
        Self {
            _ui_tx: ui_tx,
            ui_rx,
            tokio_tx,
            lang: Lang::Zh,
            current_tab: Tab::Provision,
            device_name: "Yundrone_UAV".to_string(),
            logs: vec!["[SYS] Init GUI engine...".into()],
            is_scanning: false,
            is_connected: false,
            ssid_input: String::new(),
            pwd_input: String::new(),
            wifi_list: vec![],
            command_input: r#"{"cmd":"status"}"#.to_string(),
        }
    }

    fn push_log(&mut self, text: String) {
        self.logs.push(text);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }
    
    fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("🛰");
                ui.heading(self.lang.t("app_title"));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(self.lang.t("lang_switch")).clicked() {
                        self.lang = if self.lang == Lang::Zh { Lang::En } else { Lang::Zh };
                    }
                    
                    ui.add_space(15.0);
                    let status = if self.is_connected {
                        egui::RichText::new(self.lang.t("conn_yes")).color(egui::Color32::GREEN)
                    } else if self.is_scanning {
                        egui::RichText::new(self.lang.t("conn_wait")).color(egui::Color32::YELLOW)
                    } else {
                        egui::RichText::new(self.lang.t("conn_no")).color(egui::Color32::RED)
                    };
                    ui.label(status);
                });
            });
            ui.add_space(8.0);
        });
    }

    fn render_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("control_panel").min_width(220.0).show(ctx, |ui| {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(self.lang.t("device_prefix"));
                ui.text_edit_singleline(&mut self.device_name);
                ui.add_space(10.0);
                
                ui.add_enabled_ui(!self.is_scanning && !self.is_connected, |ui| {
                    if ui.button(self.lang.t("scan_btn")).clicked() {
                        let _ = self.tokio_tx.send(BtleCommand::ScanAndConnect { 
                            prefix: self.device_name.clone(), 
                            timeout_secs: 10 
                        });
                    }
                });
            });
            
            ui.add_space(20.0);
            ui.vertical(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Provision, self.lang.t("tab_provision"));
                ui.selectable_value(&mut self.current_tab, Tab::Diagnostic, self.lang.t("tab_diag"));
                ui.selectable_value(&mut self.current_tab, Tab::Logs, self.lang.t("tab_logs"));
            });
        });
    }

    fn render_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                Tab::Provision => {
                    ui.heading(self.lang.t("tab_provision"));
                    ui.separator();
                    ui.add_space(10.0);
                    
                    ui.add_enabled_ui(self.is_connected, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(self.lang.t("ssid_label"));
                            ui.text_edit_singleline(&mut self.ssid_input);
                        });
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.label(self.lang.t("pwd_label"));
                            ui.add(egui::TextEdit::singleline(&mut self.pwd_input).password(true));
                        });
                        
                        ui.add_space(15.0);
                        ui.horizontal(|ui| {
                            if ui.button(self.lang.t("scan_ap_btn")).clicked() {
                                let _ = self.tokio_tx.send(BtleCommand::SendPayload { cmd_str: r#"{"cmd":"wifi.scan"}"#.into() });
                            }
                            if ui.button(self.lang.t("prov_btn")).clicked() {
                                let payload = format!("{{\"cmd\":\"provision\",\"args\":{{\"ssid\":\"{}\",\"pwd\":\"{}\"}}}}", self.ssid_input, self.pwd_input);
                                let _ = self.tokio_tx.send(BtleCommand::SendPayload { cmd_str: payload });
                            }
                        });

                        ui.add_space(15.0);
                        if !self.wifi_list.is_empty() {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                egui::Grid::new("wifi_table").striped(true).show(ui, |ui| {
                                    ui.heading(self.lang.t("col_ssid"));
                                    ui.heading(self.lang.t("col_signal"));
                                    ui.heading(self.lang.t("col_channel"));
                                    ui.end_row();

                                    for ap in &self.wifi_list {
                                        let ssid = ap.get("ssid").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                        let signal = ap.get("signal").and_then(|v| v.as_i64()).unwrap_or(0);
                                        let channel = ap.get("channel").and_then(|v| v.as_i64()).unwrap_or(0);
                                        
                                        if ui.selectable_label(self.ssid_input == ssid, ssid).clicked() {
                                            self.ssid_input = ssid.to_string();
                                        }
                                        ui.label(format!("{} dBm", signal));
                                        ui.label(channel.to_string());
                                        ui.end_row();
                                    }
                                });
                            });
                        }
                    });
                }
                Tab::Diagnostic => {
                    ui.heading(self.lang.t("tab_diag"));
                    ui.separator();
                    ui.add_space(10.0);
                    
                    ui.add_enabled_ui(self.is_connected, |ui| {
                        if ui.button(self.lang.t("cmd_whoami")).clicked() {
                            let _ = self.tokio_tx.send(BtleCommand::SendPayload { cmd_str: r#"{"cmd":"sys.whoami"}"#.into() });
                        }
                        ui.add_space(5.0);
                        if ui.button(self.lang.t("cmd_ping")).clicked() {
                            let _ = self.tokio_tx.send(BtleCommand::SendPayload { cmd_str: r#"{"cmd":"ping"}"#.into() });
                        }
                        ui.add_space(5.0);
                        if ui.button(self.lang.t("cmd_help")).clicked() {
                            let _ = self.tokio_tx.send(BtleCommand::SendPayload { cmd_str: r#"{"cmd":"help"}"#.into() });
                        }
                        ui.add_space(5.0);
                        if ui.button(self.lang.t("cmd_shutdown")).clicked() {
                            let _ = self.tokio_tx.send(BtleCommand::SendPayload { cmd_str: r#"{"cmd":"shutdown"}"#.into() });
                        }
                    });
                }
                Tab::Logs => {
                    ui.heading(self.lang.t("tab_logs"));
                    ui.separator();
                    
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(ui.available_height() - 40.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in &self.logs {
                                ui.monospace(line);
                            }
                        });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(self.lang.t("raw_send"));
                        ui.add(egui::TextEdit::singleline(&mut self.command_input).font(egui::TextStyle::Monospace));
                        if ui.button(self.lang.t("btn_send")).clicked() {
                            let _ = self.tokio_tx.send(BtleCommand::SendPayload { cmd_str: self.command_input.clone() });
                        }
                    });
                }
            }
        });
    }
}

impl eframe::App for GatewayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Drain incoming background thread events
        while let Ok(event) = self.ui_rx.try_recv() {
            match event {
                AppEvent::Log(text) => {
                    // Quick heuristic hack for now: if text is a JSON payload, parse it into wifi_list
                    if text.contains(r#""cmd":"wifi.scan""#) && text.trim().starts_with("<< RX: {") {
                        let json_str = text.trim_start_matches("<< RX: ").trim();
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                            if let Some(data) = json.get("data").and_then(|v| v.get("networks")).and_then(|v| v.as_array()) {
                                self.wifi_list = data.clone();
                            }
                        }
                    }
                    self.push_log(text);
                }
                AppEvent::ScanStarted => { self.is_scanning = true; }
                AppEvent::DeviceFound => { self.is_scanning = false; }
                AppEvent::Connected => { self.is_connected = true; }
                AppEvent::Error(err) => {
                    self.is_scanning = false;
                    self.push_log(format!("[ERR] {}", err));
                }
            }
        }

        // 2. Render UI Layout
        self.render_top_panel(ctx);
        self.render_side_panel(ctx);
        self.render_central_panel(ctx);

        // 3. Keep UI refreshing to catch background events
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
