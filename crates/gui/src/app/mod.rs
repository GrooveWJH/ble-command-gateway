use std::sync::mpsc::{Receiver, Sender};

use crate::ble_worker::BtleCommand;
use crate::i18n::Lang;

mod action_ui;
mod diagnostic_panel;
mod logs_panel;
pub(crate) mod model;
mod panels;
mod provision_panel;
pub(crate) mod reducer;
pub(crate) mod settings;
mod theme;

#[cfg(test)]
mod action_tests;
#[cfg(test)]
mod heartbeat_tests;
#[cfg(test)]
mod provision_tests;
#[cfg(test)]
mod tests;

use model::ActionSlot;
use model::{AppModel, ThemePreference, UiEvent};

pub struct GatewayApp {
    _ui_tx: Sender<UiEvent>,
    ui_rx: Receiver<UiEvent>,
    tokio_tx: tokio::sync::mpsc::UnboundedSender<BtleCommand>,
    model: AppModel,
}

impl GatewayApp {
    pub fn new(
        ui_tx: Sender<UiEvent>,
        ui_rx: Receiver<UiEvent>,
        tokio_tx: tokio::sync::mpsc::UnboundedSender<BtleCommand>,
        theme_preference: ThemePreference,
    ) -> Self {
        Self {
            _ui_tx: ui_tx,
            ui_rx,
            tokio_tx,
            model: AppModel {
                theme_preference,
                ..AppModel::default()
            },
        }
    }

    fn send_command(&self, slot: ActionSlot, payload: protocol::requests::CommandPayload) {
        let _ = self
            .tokio_tx
            .send(BtleCommand::SendCommand { slot, payload });
    }

    fn send_raw_payload(&self, payload: String) {
        let _ = self.tokio_tx.send(BtleCommand::SendRaw {
            slot: ActionSlot::RawSend,
            payload,
        });
    }

    fn record_local_success(&mut self, slot: ActionSlot, detail: Option<String>) {
        reducer::reduce(
            &mut self.model,
            UiEvent::ActionSucceeded {
                slot,
                request_id: None,
                detail,
            },
        );
    }

    fn toggle_lang(&mut self) {
        self.model.lang = if self.model.lang == Lang::Zh {
            Lang::En
        } else {
            Lang::Zh
        };
    }

    fn set_theme_preference(
        &mut self,
        ctx: &eframe::egui::Context,
        theme_preference: ThemePreference,
    ) {
        self.model.theme_preference = theme_preference;
        ctx.set_theme(theme_preference.to_egui());

        let settings = settings::StoredSettings { theme_preference };
        if let Err(err) = settings::save_settings(&settings) {
            self.model
                .logs
                .push(format!("[ERR] Failed to save theme settings: {err}"));
        }
    }
}

impl eframe::App for GatewayApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.ui_rx.try_recv() {
            reducer::reduce(&mut self.model, event);
        }

        self.render_top_panel(ctx);
        self.render_side_panel(ctx);
        self.render_central_panel(ctx);

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
