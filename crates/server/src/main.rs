#![cfg_attr(not(target_os = "linux"), allow(unused))]

#[cfg(all(not(target_os = "linux"), not(test), not(clippy)))]
compile_error!("The 'server' crate depends on Linux-specific APIs (BlueZ/bluer) and can ONLY be compiled for Linux targets. Please use a Linux machine or cross-compile using --target aarch64-unknown-linux-gnu");

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use futures::FutureExt;
    use tokio::sync::broadcast;
    use tracing::{info, warn};

    use bluer::gatt::local::{
        Application, Characteristic, CharacteristicNotify, CharacteristicNotifyMethod,
        CharacteristicWrite, CharacteristicWriteMethod, Service,
    };
    let runtime = server::runtime::build_runtime_context()?;

    tracing_subscriber::fmt::init();
    info!("Starting YunDrone BLE Command Gateway (Linux Server)...");

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    let advertising_capabilities = server::advertising::probe_capabilities(&adapter).await;
    let bluetoothd_environment = server::bluetoothd::inspect_bluetoothd_environment().await;
    server::runtime::log_advertising_environment(
        &adapter,
        &advertising_capabilities,
        &bluetoothd_environment,
        &runtime,
    )
    .await;

    let (notify_tx, _) = broadcast::channel::<Vec<u8>>(32);
    let write_notify_tx = notify_tx.clone();
    let read_notify_tx = notify_tx.clone();

    // We process incoming writes here. Because we used Io method, bluer will actually provide a stream of writes.
    // However, writing an async handler in bluer requires registering an Io handler, but for simplicity we can use Fun.
    // Let's redefine the Write Characteristic with a Fun handler.
    let write_char_def = Characteristic {
        uuid: runtime.write_uuid,
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, _req| {
                let tx = write_notify_tx.clone();
                Box::pin(async move {
                    match protocol::parse_request(&new_value) {
                        Ok(req) => {
                            let command_name = req.payload.command_name().to_string();
                            info!(
                                request_id = %req.id,
                                cmd = %command_name,
                                protocol_version = %req.v,
                                payload_bytes = new_value.len(),
                                "ble.request.received"
                            );

                            let result =
                                server::services::run_payload_command(&req.payload, 30.0).await;

                            let resp = protocol::CommandResponse {
                                id: req.id.clone(),
                                ok: result.ok,
                                code: result.code,
                                text: result.text,
                                data: result.data,
                                v: protocol::PROTOCOL_VERSION.into(),
                            };

                            let response_code = resp.code.clone();
                            let response_ok = resp.ok;
                            let response_bytes = protocol::encode_response(&resp)
                                .map(|value| value.len())
                                .ok();
                            let chunks = protocol::chunking::chunk_response(resp);
                            let chunk_count = chunks.len();
                            let mut max_chunk_bytes = 0usize;

                            for chunk in chunks {
                                match protocol::encode_response(&chunk) {
                                    Ok(ser) => {
                                        max_chunk_bytes = max_chunk_bytes.max(ser.len());
                                        let _ = tx.send(ser);
                                    }
                                    Err(err) => {
                                        warn!(
                                            request_id = %req.id,
                                            cmd = %command_name,
                                            error = %err,
                                            "ble.response.encode_failed"
                                        );
                                    }
                                }
                            }

                            info!(
                                request_id = %req.id,
                                cmd = %command_name,
                                response_code = %response_code,
                                response_ok,
                                chunk_count,
                                response_bytes,
                                max_chunk_bytes,
                                "ble.response.sent"
                            );
                        }
                        Err(e) => warn!(
                            error = %e,
                            payload_bytes = new_value.len(),
                            "ble.request.parse_failed"
                        ),
                    }
                    Ok(())
                })
            })),
            ..Default::default()
        }),
        ..Default::default()
    };

    let read_char_def = Characteristic {
        uuid: runtime.read_uuid,
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                let mut rx = read_notify_tx.subscribe();
                async move {
                    tokio::spawn(async move {
                        info!("ble.notify.subscribed");
                        loop {
                            match rx.recv().await {
                                Ok(value) => {
                                    if let Err(err) = notifier.notify(value).await {
                                        warn!(error = %err, "ble.notify.failed");
                                        break;
                                    }
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                    warn!(skipped, "ble.notify.lagged");
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                            }
                        }
                        info!("ble.notify.closed");
                    });
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    };

    let app = Application {
        services: vec![Service {
            uuid: runtime.service_uuid,
            primary: true,
            characteristics: vec![write_char_def, read_char_def],
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut advertising_session = server::runtime::start_advertising(
        &adapter,
        &advertising_capabilities,
        &runtime,
        server::advertising::AdvertisingPhase::FastStart,
    )
    .await?;

    let _app_handle = adapter.serve_gatt_application(app).await?;
    info!(
        adapter_name = %adapter.name(),
        service_uuid = %runtime.service_uuid,
        write_uuid = %runtime.write_uuid,
        read_uuid = %runtime.read_uuid,
        "ble.gatt.ready"
    );

    let reset_delay = tokio::time::sleep(runtime.advertising_policy.fast_duration);
    tokio::pin!(reset_delay);
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = &mut reset_delay => {
            advertising_session.stop().await?;
            advertising_session = server::runtime::start_advertising(
                &adapter,
                &advertising_capabilities,
                &runtime,
                server::advertising::AdvertisingPhase::Steady,
            ).await?;
            tokio::signal::ctrl_c().await?;
        }
    }

    advertising_session.stop().await?;
    info!(adapter_name = %adapter.name(), "ble.server.stopping");

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    // This empty main is compiled only when OS is not Linux,
    // so `cargo build` doesn't complain about missing `main` before hitting the `compile_error!`.
}
