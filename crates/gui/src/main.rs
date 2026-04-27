#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ble_worker;
mod i18n;

use eframe::egui;
use std::sync::mpsc::channel;

const GUI_RUNTIME: platform_runtime::AppRuntime = platform_runtime::AppRuntime {
    bundle_name: "YunDrone BLE Gateway.app",
    executable_name: "gui",
    info_plist: include_bytes!("../macos/Info.plist"),
};

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Cross-platform typical system CJK fonts
    let font_paths = [
        "/System/Library/Fonts/STHeiti Light.ttc",    // macOS
        "/System/Library/Fonts/PingFang.ttc",         // macOS alt
        "/System/Library/Fonts/Hiragino Sans GB.ttc", // macOS alt 2
        "C:\\Windows\\Fonts\\msyh.ttc",               // Windows
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", // Linux
        "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc", // Linux
    ];

    for path in font_paths.into_iter() {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk_fallback".to_owned(),
                egui::FontData::from_owned(font_data),
            );

            // Inject CJK font down the priority list just behind the default sans-serifs
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk_fallback".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk_fallback".to_owned());
            break; // Stop after finding the first valid system font
        }
    }

    ctx.set_fonts(fonts);

    // Boost typography scale to fill out emptiness
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(22.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Name("Title".into()),
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(16.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::new(14.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(16.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(12.0, egui::FontFamily::Proportional),
        ),
    ]
    .into();

    // Add some soft padding globally
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    // 0. Initialize logging
    tracing_subscriber::fmt::init();

    if platform_runtime::prepare_gui_runtime(&GUI_RUNTIME)
        .map_err(|err| eframe::Error::AppCreation(err.into()))?
        == platform_runtime::RuntimeLaunchOutcome::Relaunched
    {
        return Ok(());
    }

    // 1. Create communication channels between UI Thread & Async Worker
    let (ui_tx, ui_rx) = channel::<app::model::UiEvent>();
    let (tokio_tx, tokio_rx) = tokio::sync::mpsc::unbounded_channel::<ble_worker::BtleCommand>();

    // 2. Spawn Background BLE tokio worker engine
    ble_worker::spawn_btle_worker(ui_tx.clone(), tokio_rx);

    // 3. Configure and start Native GUI window
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 520.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "YunDrone BLE Gateway",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            let settings = app::settings::load_settings().unwrap_or_default();
            cc.egui_ctx.set_theme(settings.theme_preference.to_egui());
            Ok(Box::new(app::GatewayApp::new(
                ui_tx,
                ui_rx,
                tokio_tx,
                settings.theme_preference,
            )))
        }),
    )
}
