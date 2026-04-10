use std::sync::mpsc::{Receiver, Sender};

use crate::ble_worker::BtleCommand;
use crate::i18n::Lang;

pub(crate) mod model;
mod panels;
pub(crate) mod reducer;

#[cfg(test)]
mod tests;

use model::{AppModel, UiEvent};

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
    ) -> Self {
        Self {
            _ui_tx: ui_tx,
            ui_rx,
            tokio_tx,
            model: AppModel::default(),
        }
    }

    fn send_command(&self, payload: protocol::requests::CommandPayload) {
        let _ = self.tokio_tx.send(BtleCommand::SendCommand { payload });
    }

    fn toggle_lang(&mut self) {
        self.model.lang = if self.model.lang == Lang::Zh {
            Lang::En
        } else {
            Lang::Zh
        };
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
