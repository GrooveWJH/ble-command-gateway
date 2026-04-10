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
    use uuid::Uuid;

    let advertised_name =
        server::device_name::generate_device_name(protocol::config::DEFAULT_DEVICE_NAME);
    let advertising_policy = server::advertising::default_policy();

    // Protocol Definitions
    let service_uuid = Uuid::parse_str("6E400001-B5A3-F393-E0A9-E50E24DCCA9E")?;
    let write_uuid = Uuid::parse_str("6E400002-B5A3-F393-E0A9-E50E24DCCA9E")?;
    let read_uuid = Uuid::parse_str("6E400003-B5A3-F393-E0A9-E50E24DCCA9E")?;

    tracing_subscriber::fmt::init();
    info!("Starting YunDrone BLE Command Gateway (Linux Server)...");

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    let adapter_name = adapter.name();
    let advertising_capabilities = server::advertising::probe_capabilities(&adapter).await;

    info!(
        adapter_name = %adapter_name,
        advertised_name = %advertised_name,
        "ble.server.starting"
    );

    let (notify_tx, _) = broadcast::channel::<Vec<u8>>(32);
    let write_notify_tx = notify_tx.clone();
    let read_notify_tx = notify_tx.clone();

    // We process incoming writes here. Because we used Io method, bluer will actually provide a stream of writes.
    // However, writing an async handler in bluer requires registering an Io handler, but for simplicity we can use Fun.
    // Let's redefine the Write Characteristic with a Fun handler.
    let write_char_def = Characteristic {
        uuid: write_uuid,
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
                            let chunks = protocol::chunking::chunk_response(resp);
                            let chunk_count = chunks.len();

                            for chunk in chunks {
                                match protocol::encode_response(&chunk) {
                                    Ok(ser) => {
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
        uuid: read_uuid,
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
            uuid: service_uuid,
            primary: true,
            characteristics: vec![write_char_def, read_char_def],
            ..Default::default()
        }],
        ..Default::default()
    };

    info!(
        adapter_name = %adapter_name,
        active_instances = ?advertising_capabilities.active_instances,
        supported_instances = ?advertising_capabilities.supported_instances,
        max_advertisement_length = ?advertising_capabilities.max_advertisement_length,
        max_scan_response_length = ?advertising_capabilities.max_scan_response_length,
        max_tx_power = ?advertising_capabilities.max_tx_power,
        can_set_tx_power = advertising_capabilities.can_set_tx_power,
        secondary_channels = ?advertising_capabilities.secondary_channels,
        platform_features = ?advertising_capabilities.platform_features,
        "ble.advertising.capabilities"
    );
    warn!(
        adapter_name = %adapter_name,
        advertised_name = %advertised_name,
        payload_hint = %server::advertising::payload_risk_hint(
            &advertised_name,
            &advertising_capabilities
        ),
        "ble.advertising.payload_risk"
    );
    warn!(
        adapter_name = %adapter_name,
        "ble.advertising.interval_unverified"
    );

    let fast_config = server::advertising::applied_config(
        &advertising_policy,
        server::advertising::AdvertisingPhase::FastStart,
        &advertising_capabilities,
    );
    let mut adv_handle =
        server::advertising::advertise_phase(&adapter, &advertised_name, service_uuid, fast_config)
            .await?;
    info!(
        adapter_name = %adapter_name,
        advertised_name = %advertised_name,
        phase = server::advertising::phase_name(fast_config.phase),
        fast_duration_secs = advertising_policy.fast_duration.as_secs(),
        "ble.advertising.fast_start"
    );
    info!(
        adapter_name = %adapter_name,
        advertised_name = %advertised_name,
        phase = server::advertising::phase_name(fast_config.phase),
        min_interval = %server::advertising::interval_ms_text(fast_config.interval.min),
        max_interval = %server::advertising::interval_ms_text(fast_config.interval.max),
        tx_power = ?fast_config.tx_power,
        "ble.advertising.ready"
    );
    info!(
        adapter_name = %adapter_name,
        advertised_name = %advertised_name,
        phase = server::advertising::phase_name(fast_config.phase),
        min_interval = %server::advertising::interval_ms_text(fast_config.interval.min),
        max_interval = %server::advertising::interval_ms_text(fast_config.interval.max),
        tx_power = ?fast_config.tx_power,
        "ble.advertising.config_applied"
    );

    let _app_handle = adapter.serve_gatt_application(app).await?;
    info!(
        adapter_name = %adapter_name,
        service_uuid = %service_uuid,
        write_uuid = %write_uuid,
        read_uuid = %read_uuid,
        "ble.gatt.ready"
    );

    let reset_delay = tokio::time::sleep(advertising_policy.fast_duration);
    tokio::pin!(reset_delay);
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = &mut reset_delay => {
            drop(adv_handle);
            let steady_config = server::advertising::applied_config(
                &advertising_policy,
                server::advertising::AdvertisingPhase::Steady,
                &advertising_capabilities,
            );
            adv_handle = server::advertising::advertise_phase(
                &adapter,
                &advertised_name,
                service_uuid,
                steady_config,
            ).await?;
            info!(
                adapter_name = %adapter_name,
                advertised_name = %advertised_name,
                phase = server::advertising::phase_name(steady_config.phase),
                min_interval = %server::advertising::interval_ms_text(steady_config.interval.min),
                max_interval = %server::advertising::interval_ms_text(steady_config.interval.max),
                tx_power = ?steady_config.tx_power,
                "ble.advertising.reset_to_steady"
            );
            info!(
                adapter_name = %adapter_name,
                advertised_name = %advertised_name,
                phase = server::advertising::phase_name(steady_config.phase),
                min_interval = %server::advertising::interval_ms_text(steady_config.interval.min),
                max_interval = %server::advertising::interval_ms_text(steady_config.interval.max),
                tx_power = ?steady_config.tx_power,
                "ble.advertising.config_applied"
            );
            tokio::signal::ctrl_c().await?;
        }
    }

    drop(adv_handle);
    info!(adapter_name = %adapter_name, "ble.server.stopping");

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    // This empty main is compiled only when OS is not Linux,
    // so `cargo build` doesn't complain about missing `main` before hitting the `compile_error!`.
}
