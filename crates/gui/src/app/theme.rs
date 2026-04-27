use eframe::egui;

pub(super) fn panel_frame(ui: &egui::Ui) -> egui::Frame {
    let visuals = ui.visuals();
    egui::Frame::none()
        .fill(visuals.faint_bg_color)
        .stroke(visuals.widgets.noninteractive.bg_stroke)
        .inner_margin(egui::Margin::same(10.0))
        .rounding(egui::Rounding::same(6.0))
}

pub(super) fn success_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(120, 220, 130)
    } else {
        egui::Color32::from_rgb(30, 135, 55)
    }
}

pub(super) fn failure_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(255, 120, 120)
    } else {
        egui::Color32::from_rgb(185, 45, 60)
    }
}

pub(super) fn warning_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(255, 190, 90)
    } else {
        egui::Color32::from_rgb(176, 104, 12)
    }
}

pub(super) fn log_text_color(ui: &egui::Ui) -> egui::Color32 {
    ui.visuals().text_color()
}
