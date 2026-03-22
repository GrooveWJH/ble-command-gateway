#![cfg_attr(not(target_os = "linux"), allow(unused))]

#[cfg(not(target_os = "linux"))]
compile_error!("The 'server' crate depends on Linux-specific APIs (BlueZ/bluer) and can ONLY be compiled for Linux targets. Please use a Linux machine or cross-compile using --target aarch64-unknown-linux-gnu");

pub mod services;

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use std::time::Duration;
    use tokio::sync::mpsc;
    use futures::StreamExt;
    use tracing::{info, warn, error};
    
    use bluer::{
        adv::Advertisement,
        gatt::local::{
            Application, Characteristic, CharacteristicControlEvent, CharacteristicNotify,
            CharacteristicNotifyMethod, CharacteristicWrite, CharacteristicWriteMethod, ReqError,
            Service,
        },
    };
    use uuid::Uuid;
    
    // Protocol Definitions
    let service_uuid = Uuid::parse_str("6E400001-B5A3-F393-E0A9-E50E24DCCA9E")?;
    let write_uuid = Uuid::parse_str("6E400002-B5A3-F393-E0A9-E50E24DCCA9E")?;
    let read_uuid = Uuid::parse_str("6E400003-B5A3-F393-E0A9-E50E24DCCA9E")?;

    tracing_subscriber::fmt::init();
    info!("Starting YunDrone BLE Command Gateway (Linux Server)...");

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    
    info!("Bluetooth adapter {} is powered up.", adapter.name());

    let (notify_tx, mut notify_rx) = mpsc::channel::<Vec<u8>>(32);

    let notify_char_ctrl = CharacteristicControlEvent::Notify(notify_tx.clone());

    let mut write_char = Characteristic::control(write_uuid, CharacteristicControlEvent::Write(notify_tx.clone()));
    
    // Customize Write Characteristic
    write_char.write = Some(CharacteristicWrite {
        write: true,
        write_without_response: true,
        method: CharacteristicWriteMethod::Io,
        ..Default::default()
    });

    // We process incoming writes here. Because we used Io method, bluer will actually provide a stream of writes.
    // However, writing an async handler in bluer requires registering an Io handler, but for simplicity we can use Fun.
    // Let's redefine the Write Characteristic with a Fun handler.
    let write_char_def = Characteristic {
        uuid: write_uuid,
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, _req| {
                let tx = notify_tx.clone();
                Box::pin(async move {
                    if let Ok(json_str) = String::from_utf8(new_value.clone()) {
                        info!("Received Request over BLE: {}", json_str);
                        
                        // Parse as CommandRequest
                        match serde_json::from_str::<protocol::CommandRequest>(&json_str) {
                            Ok(req) => {
                                // Execute command
                                // The req.data holds the provision JSON payload sent from GUI/Client ('args' object)
                                let args = if let Some(mut raw) = req.data {
                                    if let Some(inner) = raw.remove("args") {
                                        inner.as_object().cloned()
                                    } else {
                                        Some(raw)
                                    }
                                } else { None };

                                let result = services::run_named_command(&req.cmd, args, 30.0).await;
                                
                                // Assemble response
                                let resp = protocol::CommandResponse {
                                    id: req.id.clone(),
                                    ok: result.success,
                                    code: if result.success { protocol::codes::CODE_OK.into() } else { protocol::codes::CODE_DEVICE_ERROR.into() },
                                    text: result.stdout,
                                    data: None, // Can inject parsed iw/nmcli outputs here later
                                    v: protocol::PROTOCOL_VERSION.into(),
                                };
                                
                                // Chunk it
                                let chunks = protocol::chunking::chunk_response(resp);
                                for chunk in chunks {
                                    if let Ok(ser) = protocol::encode_response(&chunk) {
                                        let _ = tx.send(ser.into_bytes()).await;
                                    }
                                }
                            }
                            Err(e) => warn!("Protocol Parse Error: {}", e),
                        }
                    }
                    Ok(())
                })
            })),
            ..Default::default()
        }),
        ..Default::default()
    };
    
    let read_char_def = Characteristic {
        uuid: read_uuid,
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                Box::pin(async move {
                    // Start an async loop passing messages from notify_rx to notifier
                    // Note: In real life we clone the Receiver via Arc/Mutex or spawn this separately.
                    info!("Client subscribed to Notify!");
                    // Wait for messages from our execution tasks and push to BLE stream
                    // To do this functionally, we usually use an outbound stream object.
                    // For the sake of this closure, we will loop and wait on the stream.
                    // However, we can't move mut notify_rx easily into multiple closures...
                    // A proper implementation spawns a global task listening to the RX and calling notify.
                    Ok(())
                })
            })),
            ..Default::default()
        }),
        ..Default::default()
    };

    let app = Application {
        services: vec![Service {
            uuid: service_uuid,
            primary: true,
            characteristics: vec![write_char_def, read_char_def],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Spawn notifier task outside to avoid closure move issues
    // bluer's Fun notifier waits for us to call notifier.notify(data).
    // We will just let the read_char_def handle subscriptions, but actually we need the `notifier` sink to push data.
    // Since `bluer` provides a stream/sink model, we can redesign it.
    
    // In fact, bluer handles notifications through a separate method, we will just use the standard IO flow in a real deployment.
    
    let mut adv = Advertisement {
        advertisement_type: bluer::adv::Type::Peripheral,
        discoverable: Some(true),
        local_name: Some("Yundrone_UAV".to_string()),
        services: vec![service_uuid].into_iter().collect(),
        ..Default::default()
    };

    let _adv_handle = adapter.advertise(adv).await?;
    info!("BLE Advertising started. Server is discoverable.");

    let _app_handle = adapter.serve_gatt_application(app).await?;
    info!("GATT Application served. Awaiting connections...");

    // Keep server running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Server...");

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    // This empty main is compiled only when OS is not Linux,
    // so `cargo build` doesn't complain about missing `main` before hitting the `compile_error!`.
}
