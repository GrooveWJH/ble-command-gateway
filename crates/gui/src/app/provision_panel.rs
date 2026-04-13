use eframe::egui;
use protocol::{requests::CommandPayload, responses::WifiNetwork};

use super::action_ui::{render_action_status, render_refreshing_badge, PROVISION_ACTION_SLOTS};
use super::model::ActionSlot;
use super::GatewayApp;

impl GatewayApp {
    pub(super) fn render_provision_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.model.lang.t("tab_provision"));
        ui.separator();
        ui.add_space(8.0);
        render_action_status(ui, &self.model, &PROVISION_ACTION_SLOTS);
        ui.add_space(8.0);

        ui.add_enabled_ui(self.model.is_connected, |ui| {
            render_wifi_inputs(ui, self);
            ui.add_space(14.0);
            render_provision_buttons(ui, self);
            ui.add_space(14.0);
            render_wifi_table(ui, self);
            ui.add_space(12.0);
            render_provision_result(ui, self);
        });
    }
}

fn render_wifi_inputs(ui: &mut egui::Ui, app: &mut GatewayApp) {
    ui.horizontal(|ui| {
        ui.label(app.model.lang.t("ssid_label"));
        ui.text_edit_singleline(&mut app.model.ssid_input);
    });
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        ui.label(app.model.lang.t("pwd_label"));
        ui.add(egui::TextEdit::singleline(&mut app.model.pwd_input).password(true));
    });
}

fn render_provision_buttons(ui: &mut egui::Ui, app: &mut GatewayApp) {
    let busy = app.model.active_action.is_some();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!busy, egui::Button::new(app.model.lang.t("scan_ap_btn")))
            .clicked()
        {
            app.send_command(
                ActionSlot::WifiScan,
                CommandPayload::WifiScan { ifname: None },
            );
        }
        if ui
            .add_enabled(!busy, egui::Button::new(app.model.lang.t("prov_btn")))
            .clicked()
        {
            app.send_command(
                ActionSlot::Provision,
                CommandPayload::Provision {
                    ssid: app.model.ssid_input.clone(),
                    pwd: if app.model.pwd_input.is_empty() {
                        None
                    } else {
                        Some(app.model.pwd_input.clone())
                    },
                },
            );
        }
    });
}

fn render_wifi_table(ui: &mut egui::Ui, app: &mut GatewayApp) {
    if app.model.wifi_list.is_empty() {
        return;
    }

    let rows = aggregate_wifi_networks(&app.model.wifi_list);
    result_frame().show(ui, |ui| {
        let width = ui.available_width();
        render_wifi_header_row(ui, app, width);
        ui.separator();

        egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .auto_shrink([false, false])
            .max_height(220.0)
            .show(ui, |ui| {
                ui.set_width(width);
                for row in rows {
                    render_wifi_data_row(ui, app, &row, width);
                }
            });
    });
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WifiDisplayRow {
    pub ssid: String,
    pub signal: i32,
    pub channels: String,
    pub instance_count: usize,
}

pub(super) fn aggregate_wifi_networks(networks: &[WifiNetwork]) -> Vec<WifiDisplayRow> {
    let mut rows: Vec<WifiDisplayRow> = Vec::new();

    for network in networks {
        if let Some(row) = rows.iter_mut().find(|row| row.ssid == network.ssid) {
            row.signal = row.signal.max(network.signal);
            row.instance_count += 1;
            merge_channel(&mut row.channels, &network.channel);
            continue;
        }

        rows.push(WifiDisplayRow {
            ssid: network.ssid.clone(),
            signal: network.signal,
            channels: network.channel.clone(),
            instance_count: 1,
        });
    }

    for row in &mut rows {
        row.channels = normalize_channels(&row.channels);
    }

    rows
}

fn render_wifi_header_row(ui: &mut egui::Ui, app: &GatewayApp, width: f32) {
    let (ssid_width, signal_width, channel_width) = wifi_column_widths(width);
    ui.horizontal(|ui| {
        ui.add_sized(
            [ssid_width, 20.0],
            egui::Label::new(egui::RichText::new(app.model.lang.t("col_ssid")).strong()),
        );
        ui.add_sized(
            [signal_width, 20.0],
            egui::Label::new(egui::RichText::new(app.model.lang.t("col_signal")).strong()),
        );
        ui.add_sized(
            [channel_width, 20.0],
            egui::Label::new(egui::RichText::new(app.model.lang.t("col_channel")).strong()),
        );
    });
}

fn render_wifi_data_row(ui: &mut egui::Ui, app: &mut GatewayApp, row: &WifiDisplayRow, width: f32) {
    let (ssid_width, signal_width, channel_width) = wifi_column_widths(width);
    let selected = app.model.ssid_input == row.ssid;
    let label = if row.instance_count > 1 {
        format!("{} [x{}]", row.ssid, row.instance_count)
    } else {
        row.ssid.clone()
    };

    ui.horizontal(|ui| {
        let response = ui.add_sized(
            [ssid_width, 24.0],
            egui::Button::new(label).selected(selected),
        );
        if response.clicked() {
            app.model.ssid_input = row.ssid.clone();
        }
        ui.add_sized(
            [signal_width, 24.0],
            egui::Label::new(egui::RichText::new(format!("{} dBm", row.signal)).monospace()),
        );
        ui.add_sized(
            [channel_width, 24.0],
            egui::Label::new(egui::RichText::new(&row.channels).monospace()),
        );
    });
}

fn wifi_column_widths(total_width: f32) -> (f32, f32, f32) {
    let ssid_width = (total_width * 0.55).max(180.0);
    let signal_width = (total_width * 0.18).max(92.0);
    let channel_width = (total_width - ssid_width - signal_width - 12.0).max(120.0);
    (ssid_width, signal_width, channel_width)
}

fn merge_channel(channels: &mut String, channel: &str) {
    let mut values = channels
        .split(", ")
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if values.iter().all(|value| value != channel) {
        values.push(channel.to_string());
    }
    *channels = values.join(", ");
}

fn normalize_channels(channels: &str) -> String {
    let mut values = channels
        .split(", ")
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    values.sort_by(|left, right| channel_order(left.as_str()).cmp(&channel_order(right.as_str())));
    values.dedup();
    values.join(", ")
}

fn channel_order(value: &str) -> (Option<u16>, &str) {
    (value.parse::<u16>().ok(), value)
}

fn render_provision_result(ui: &mut egui::Ui, app: &GatewayApp) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(app.model.lang.t("prov_result_title")).strong());
        render_refreshing_badge(ui, &app.model, &PROVISION_ACTION_SLOTS);
    });
    let frame = result_frame();
    frame.show(ui, |ui| {
        if let Some(result) = &app.model.provision_result {
            let status = if result.ok {
                egui::RichText::new("OK").color(egui::Color32::from_rgb(120, 220, 130))
            } else {
                egui::RichText::new("FAIL").color(egui::Color32::from_rgb(255, 120, 120))
            };
            ui.horizontal(|ui| {
                ui.label(format!("code: {}", result.code));
                ui.label(status);
            });
            ui.label(egui::RichText::new(format!("status: {}", result.status)).monospace());
            ui.label(egui::RichText::new(format!("ssid: {}", result.ssid)).monospace());
            ui.label(
                egui::RichText::new(format!(
                    "ip: {}",
                    result.ip.clone().unwrap_or_else(|| "N/A".to_string())
                ))
                .monospace(),
            );
            ui.label(egui::RichText::new(format!("text: {}", result.text)).monospace());
        } else {
            ui.label(app.model.lang.t("prov_result_empty"));
        }
    });
}

fn result_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(22, 25, 30))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 66, 78)))
        .inner_margin(egui::Margin::same(10.0))
        .rounding(egui::Rounding::same(6.0))
}
