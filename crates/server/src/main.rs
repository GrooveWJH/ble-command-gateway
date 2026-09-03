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
    server::logging::init_logging();
    let args = server::config::parse_args();
    let name_prefix = server::config::device_prefix_from_args(&args)?;

    info!("Starting YunDrone BLE Command Gateway (Linux Server)...");

    let session = bluer::Session::new().await?;
    let (adapter, adapter_address) =
        server::runtime::wait_for_adapter(&session, args.adapter.as_deref(), &name_prefix).await?;
    let runtime = server::runtime::build_runtime_context(args, adapter_address)?;
    adapter.set_powered(true).await?;
    if let Err(err) =
        server::adapter_identity::apply_and_log_public_identity(&adapter, &runtime.identity.name)
            .await
    {
        warn!(
            adapter_name = %adapter.name(),
            identity_name = %runtime.identity.name,
            error = %err,
            "ble.adapter.identity_apply_failed"
        );
    }
    if let Err(err) = server::adapter_pairing::disable_and_log_pairing(&adapter).await {
        warn!(
            adapter_name = %adapter.name(),
            error = %err,
            "ble.adapter.pairing_disable_failed"
        );
    }
    let _pairing_guard = server::adapter_pairing::spawn_pairing_guard(adapter.clone());
    let advertising_capabilities = server::advertising::probe_capabilities(&adapter).await;
    let bluetoothd_environment = server::bluetoothd::inspect_bluetoothd_environment().await;
    let selection = server::advertising_backend::AdvertisingBackend::select(
        runtime.backend_preference,
        &advertising_capabilities,
        &bluetoothd_environment,
        server::legacy_hci::is_available(),
    );
    let (selected_backend, reason) = match selection {
        Ok(value) => value,
        Err(err) => {
            tracing::error!(
                adapter_name = %adapter.name(),
                error = %err,
                "ble.adapter.preflight_failed"
            );
            return Err(err);
        }
    };
    let mut runtime = runtime;
    runtime.advertising_backend = selected_backend;
    tracing::info!(
        requested_backend = runtime.backend_preference.as_str(),
        selected_backend = runtime.advertising_backend.as_str(),
        reason,
        "ble.advertising.backend_selected"
    );
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
    let command_events = server::command_events::CommandEventSender::new(
        write_notify_tx,
        server::services::ServiceContext::new(runtime.identity.name.clone()),
    );

    // We process incoming writes here. Because we used Io method, bluer will actually provide a stream of writes.
    // However, writing an async handler in bluer requires registering an Io handler, but for simplicity we can use Fun.
    // Let's redefine the Write Characteristic with a Fun handler.
    let write_char_def = Characteristic {
        uuid: runtime.write_uuid,
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, _req| {
                let command_events = command_events.clone();
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

                            match req.payload.clone() {
                                protocol::requests::CommandPayload::LinkAck(ack) => {
                                    command_events.handle_ack(ack, &req.id).await;
                                }
                                _ => {
                                    command_events.handle_request(req, command_name).await;
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                payload_bytes = new_value.len(),
                                "ble.request.parse_failed"
                            );
                            if let Some(response) = protocol::parse_error_response(&new_value, &e) {
                                let command_name = response
                                    .cmd
                                    .clone()
                                    .unwrap_or_else(|| "unknown".to_string());
                                command_events.send_response_event(response, &command_name);
                            }
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
    let _app_handle = adapter.serve_gatt_application(app).await?;
    info!(
        adapter_name = %adapter.name(),
        service_uuid = %runtime.service_uuid,
        write_uuid = %runtime.write_uuid,
        read_uuid = %runtime.read_uuid,
        "ble.gatt.ready"
    );

    let mut advertising_session = server::runtime::start_advertising(
        &adapter,
        &advertising_capabilities,
        &mut runtime,
        server::advertising::AdvertisingPhase::FastStart,
    )
    .await?;

    let reset_delay = tokio::time::sleep(runtime.advertising_policy.fast_duration);
    tokio::pin!(reset_delay);
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = &mut reset_delay => {
            advertising_session.stop().await?;
            advertising_session = server::runtime::start_advertising(
                &adapter,
                &advertising_capabilities,
                &mut runtime,
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
