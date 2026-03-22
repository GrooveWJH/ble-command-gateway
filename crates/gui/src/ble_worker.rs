use std::sync::mpsc::Sender;
use std::thread;
use client::BleClient;

/// Messages sent from the background Tokio thread to the GUI UI thread
pub enum AppEvent {
    Log(String),
    ScanStarted,
    DeviceFound,
    Connected,
    Error(String),
}

/// Commands sent from the GUI UI thread to the background Tokio thread
pub enum BtleCommand {
    ScanAndConnect { prefix: String, timeout_secs: u64 },
    SendPayload { cmd_str: String },
}

/// Spawns the background tokio engine processing Bluetooth tasks
pub fn spawn_btle_worker(ui_tx: Sender<AppEvent>, mut tokio_rx: tokio::sync::mpsc::UnboundedReceiver<BtleCommand>) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut active_client: Option<BleClient> = None;
            let mut active_device = None;
            let mut write_channel = None;

            while let Some(cmd) = tokio_rx.recv().await {
                match cmd {
                    BtleCommand::ScanAndConnect { prefix, timeout_secs } => {
                        ui_tx.send(AppEvent::ScanStarted).ok();
                        ui_tx.send(AppEvent::Log(format!("[SYS] Scanning for '{}'...", prefix))).ok();
                        
                        match BleClient::new().await {
                            Ok(client) => {
                                match client.scan_for_device(&prefix, timeout_secs).await {
                                    Ok(device) => {
                                        ui_tx.send(AppEvent::DeviceFound).ok();
                                        ui_tx.send(AppEvent::Log(format!("[SYS] Connecting to MAC..."))).ok();
                                        
                                        if let Err(e) = client.connect_to_device(&device).await {
                                            ui_tx.send(AppEvent::Error(format!("Connect failed: {}", e))).ok();
                                            continue;
                                        }

                                        if let Ok((w, r)) = client.discover_characteristics(&device).await {
                                            let _ = client.subscribe_notifications(&device, &r).await;
                                            write_channel = Some(w);
                                            active_device = Some(device);
                                            active_client = Some(client);
                                            ui_tx.send(AppEvent::Connected).ok();
                                            ui_tx.send(AppEvent::Log("[SYS] Handshake complete. MTU synced.".to_string())).ok();
                                        } else {
                                            ui_tx.send(AppEvent::Error("UUID char missing".to_string())).ok();
                                        }
                                    }
                                    Err(e) => { ui_tx.send(AppEvent::Error(e.to_string())).ok(); }
                                }
                            }
                            Err(e) => { ui_tx.send(AppEvent::Error(e.to_string())).ok(); }
                        }
                    }
                    BtleCommand::SendPayload { cmd_str } => {
                        if let (Some(client), Some(dev), Some(w)) = (&active_client, &active_device, &write_channel) {
                            ui_tx.send(AppEvent::Log(format!(">> TX: {}", cmd_str))).ok();
                            if let Err(e) = client.send_payload(dev, w, cmd_str.as_bytes()).await {
                                ui_tx.send(AppEvent::Error(format!("Write fail: {}", e))).ok();
                            }
                        } else {
                            ui_tx.send(AppEvent::Error("Not connected".into())).ok();
                        }
                    }
                }
            }
        });
    });
}
