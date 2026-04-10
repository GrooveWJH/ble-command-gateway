mod handlers;

#[cfg(test)]
mod tests;

use client::{BleClient, ScannedDevice};
use std::sync::mpsc::Sender;
use std::thread;

use crate::app::model::UiEvent;

pub enum BtleCommand {
    ScanCandidates {
        prefix: String,
        timeout_secs: u64,
    },
    ConnectToCandidate {
        name: String,
    },
    SendCommand {
        payload: protocol::requests::CommandPayload,
    },
    SendRaw {
        payload: String,
    },
}

#[derive(Default)]
pub(super) struct WorkerState {
    discovered_client: Option<BleClient>,
    discovered_devices: Vec<ScannedDevice>,
    active_session: Option<client::BleSession>,
}

impl WorkerState {
    fn store_scan_results(&mut self, client: BleClient, devices: Vec<ScannedDevice>) {
        self.discovered_client = Some(client);
        self.discovered_devices = devices;
        self.active_session = None;
    }

    fn take_connection_target(&mut self, name: &str) -> Result<(BleClient, ScannedDevice), String> {
        let client = self
            .discovered_client
            .take()
            .ok_or_else(|| "No scanned devices available".to_string())?;
        let device = self
            .discovered_devices
            .iter()
            .find(|candidate| candidate.info.name == name)
            .cloned()
            .ok_or_else(|| format!("Scanned device '{}' is no longer available", name))?;
        Ok((client, device))
    }

    fn restore_client(&mut self, client: BleClient) {
        self.discovered_client = Some(client);
    }

    fn activate_session(&mut self, session: client::BleSession) {
        self.active_session = Some(session);
        self.discovered_client = None;
        self.discovered_devices.clear();
    }

    fn active_session_mut(&mut self) -> Option<&mut client::BleSession> {
        self.active_session.as_mut()
    }

    fn active_session(&self) -> Option<&client::BleSession> {
        self.active_session.as_ref()
    }
}

pub fn spawn_btle_worker(
    ui_tx: Sender<UiEvent>,
    mut tokio_rx: tokio::sync::mpsc::UnboundedReceiver<BtleCommand>,
) {
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("create BLE runtime");
        runtime.block_on(async move {
            let mut state = WorkerState::default();

            while let Some(command) = tokio_rx.recv().await {
                match command {
                    BtleCommand::ScanCandidates {
                        prefix,
                        timeout_secs,
                    } => {
                        handlers::handle_scan_candidates(&ui_tx, &mut state, prefix, timeout_secs)
                            .await
                    }
                    BtleCommand::ConnectToCandidate { name } => {
                        handlers::handle_connect_to_candidate(&ui_tx, &mut state, name).await
                    }
                    BtleCommand::SendCommand { payload } => {
                        handlers::handle_send_command(&ui_tx, &mut state, payload).await
                    }
                    BtleCommand::SendRaw { payload } => {
                        handlers::handle_send_raw(&ui_tx, &mut state, payload).await
                    }
                }
            }
        });
    });
}
