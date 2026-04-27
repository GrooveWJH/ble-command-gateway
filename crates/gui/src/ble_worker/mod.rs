mod events;
mod handlers;
mod heartbeat;
mod raw_actions;
pub(crate) mod state;

#[cfg(test)]
mod tests;

use client::BleClient;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::time::MissedTickBehavior;

use crate::app::model::{ActionSlot, UiEvent};
use state::{ScanWorkerEvent, WorkerState};

const HEARTBEAT_INTERVAL_SECS: u64 = 5;

pub enum BtleCommand {
    ScanCandidates {
        prefix: String,
        timeout_secs: u64,
    },
    StopScan,
    ConnectToCandidate {
        name: String,
    },
    Disconnect,
    SendCommand {
        slot: ActionSlot,
        payload: protocol::requests::CommandPayload,
    },
    SendRaw {
        slot: ActionSlot,
        payload: String,
    },
}

pub fn spawn_btle_worker(
    ui_tx: Sender<UiEvent>,
    mut tokio_rx: tokio::sync::mpsc::UnboundedReceiver<BtleCommand>,
) {
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("create BLE runtime");
        runtime.block_on(async move {
            let mut state = WorkerState::default();
            let (scan_event_tx, mut scan_event_rx) = mpsc::unbounded_channel::<ScanWorkerEvent>();
            let mut heartbeat_interval =
                tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
            heartbeat_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    Some(command) = tokio_rx.recv() => {
                        match command {
                            BtleCommand::ScanCandidates { prefix, timeout_secs } => {
                                if state.cancel_scan() {
                                    let _ = ui_tx.send(UiEvent::ScanStopped);
                                    let _ = ui_tx.send(UiEvent::ActionSucceeded {
                                        slot: ActionSlot::DeviceScan,
                                        request_id: None,
                                        detail: Some("stopped".to_string()),
                                    });
                                }
                                let _ = ui_tx.send(UiEvent::ActionStarted {
                                    slot: ActionSlot::DeviceScan,
                                    request_id: None,
                                });
                                let _ = ui_tx.send(UiEvent::ScanStarted);
                                let _ = ui_tx.send(UiEvent::Log(events::scan_started_log(&prefix)));
                                start_scan(&ui_tx, &mut state, &scan_event_tx, prefix, timeout_secs).await;
                            }
                            BtleCommand::StopScan => {
                                if state.cancel_scan() {
                                    let _ = ui_tx.send(UiEvent::ScanStopped);
                                    let _ = ui_tx.send(UiEvent::ActionSucceeded {
                                        slot: ActionSlot::DeviceScan,
                                        request_id: None,
                                        detail: Some("stopped".to_string()),
                                    });
                                    let _ = ui_tx.send(UiEvent::Log(events::scan_stopped_log()));
                                }
                            }
                            BtleCommand::ConnectToCandidate { name } => {
                                if state.cancel_scan() {
                                    let _ = ui_tx.send(UiEvent::ScanStopped);
                                    let _ = ui_tx.send(UiEvent::ActionSucceeded {
                                        slot: ActionSlot::DeviceScan,
                                        request_id: None,
                                        detail: Some("stopped".to_string()),
                                    });
                                    let _ = ui_tx.send(UiEvent::Log(events::connect_selected_log(&name)));
                                }
                                handlers::handle_connect_to_candidate(&ui_tx, &mut state, name).await;
                            }
                            BtleCommand::Disconnect => {
                                handlers::handle_disconnect(&ui_tx, &mut state).await;
                            }
                            BtleCommand::SendCommand { slot, payload } => {
                                handlers::handle_send_command(&ui_tx, &mut state, slot, payload).await;
                            }
                            BtleCommand::SendRaw { slot, payload } => {
                                raw_actions::handle_send_raw(&ui_tx, &mut state, slot, payload).await;
                            }
                        }
                    }
                    Some(event) = scan_event_rx.recv() => {
                        apply_scan_event(&ui_tx, &mut state, event);
                    }
                    _ = heartbeat_interval.tick(), if state.has_active_session() => {
                        heartbeat::handle_heartbeat(&ui_tx, &mut state).await;
                    }
                    else => break,
                }
            }
        });
    });
}

async fn start_scan(
    ui_tx: &Sender<UiEvent>,
    state: &mut WorkerState,
    scan_event_tx: &mpsc::UnboundedSender<ScanWorkerEvent>,
    prefix: String,
    timeout_secs: u64,
) {
    match BleClient::new().await {
        Ok(client) => {
            let (cancel_tx, mut cancel_rx) = watch::channel(false);
            let scan_id = state.begin_scan(client.clone(), cancel_tx);
            let scan_event_tx = scan_event_tx.clone();
            tokio::spawn(async move {
                let result = client
                    .scan_candidates_live(
                        &prefix,
                        timeout_secs,
                        &mut cancel_rx,
                        |event| {
                            let _ =
                                scan_event_tx.send(ScanWorkerEvent::Progress { scan_id, event });
                        },
                        |device| {
                            let _ =
                                scan_event_tx.send(ScanWorkerEvent::Candidate { scan_id, device });
                        },
                    )
                    .await;

                match result {
                    Ok(summary) => {
                        let _ = scan_event_tx.send(ScanWorkerEvent::Finished { scan_id, summary });
                    }
                    Err(error) => {
                        let _ = scan_event_tx.send(ScanWorkerEvent::Failed {
                            scan_id,
                            error: error.to_string(),
                        });
                    }
                }
            });
        }
        Err(err) => {
            let _ = ui_tx.send(UiEvent::ActionFailed {
                slot: ActionSlot::DeviceScan,
                request_id: None,
                error: err.to_string(),
            });
            let _ = ui_tx.send(UiEvent::Error(err.to_string()));
        }
    }
}

fn apply_scan_event(ui_tx: &Sender<UiEvent>, state: &mut WorkerState, event: ScanWorkerEvent) {
    match event {
        ScanWorkerEvent::Progress { scan_id, event } => {
            if state.is_active_scan(scan_id) {
                let _ = ui_tx.send(UiEvent::Log(events::scan_progress_log(&event)));
            }
        }
        ScanWorkerEvent::Candidate { scan_id, device } => {
            if state.is_active_scan(scan_id) {
                state.add_discovered_device(device.clone());
                let _ = ui_tx.send(UiEvent::ScanCandidateDiscovered(device.info));
            }
        }
        ScanWorkerEvent::Finished { scan_id, summary } => {
            if state.finish_scan(scan_id) {
                if summary.candidate_count == 0 {
                    let _ = ui_tx.send(UiEvent::ActionFailed {
                        slot: ActionSlot::DeviceScan,
                        request_id: None,
                        error: "Device not found after scan window elapsed".to_string(),
                    });
                    let _ = ui_tx.send(UiEvent::Error(
                        "Device not found after scan window elapsed".to_string(),
                    ));
                } else {
                    let _ = ui_tx.send(UiEvent::ActionSucceeded {
                        slot: ActionSlot::DeviceScan,
                        request_id: None,
                        detail: events::scan_completed_detail(summary.candidate_count),
                    });
                    let _ = ui_tx.send(UiEvent::Log(events::scan_completed_log(
                        summary.named_device_count,
                        summary.candidate_count,
                    )));
                    let _ = ui_tx.send(UiEvent::ScanFinished);
                }
            }
        }
        ScanWorkerEvent::Failed { scan_id, error } => {
            if state.finish_scan(scan_id) {
                let _ = ui_tx.send(UiEvent::ActionFailed {
                    slot: ActionSlot::DeviceScan,
                    request_id: None,
                    error: error.clone(),
                });
                let _ = ui_tx.send(UiEvent::Error(error));
            }
        }
    }
}
