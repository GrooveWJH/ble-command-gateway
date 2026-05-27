#![cfg_attr(not(target_os = "linux"), allow(unused))]

#[cfg(all(not(target_os = "linux"), not(test), not(clippy)))]
compile_error!("The 'server' crate depends on Linux-specific APIs (BlueZ/bluer) and can ONLY be compiled for Linux targets. Please use a Linux machine or cross-compile using --target aarch64-unknown-linux-gnu");

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use futures::FutureExt;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };
    use tokio::sync::broadcast;
    use tracing::{info, warn};

    use bluer::gatt::local::{
        Application, Characteristic, CharacteristicNotify, CharacteristicNotifyMethod,
        CharacteristicWrite, CharacteristicWriteMethod, Service,
    };
    server::logging::init_logging();
    let args = server::config::parse_args();
    let runtime = server::runtime::build_runtime_context(args)?;

    info!("Starting YunDrone BLE Command Gateway (Linux Server)...");

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
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
        write_notify_tx.clone(),
        server::services::ServiceContext::new(runtime.identity.name.clone()),
    );
    let write_router = std::sync::Arc::new(tokio::sync::Mutex::new(
        server::transport::WriteRouter::new(),
    ));
    let notify_session_seq = Arc::new(AtomicU64::new(0));
    let write_router_for_write = write_router.clone();
    let write_router_for_notify = write_router.clone();
    let link_diagnostics = server::ble_diagnostics::BleLinkDiagnostics::default();
    let link_diagnostics_for_write = link_diagnostics.clone();
    let link_diagnostics_for_notify = link_diagnostics.clone();

    // We process incoming writes here. Because we used Io method, bluer will actually provide a stream of writes.
    // However, writing an async handler in bluer requires registering an Io handler, but for simplicity we can use Fun.
    // Let's redefine the Write Characteristic with a Fun handler.
    let write_char_def = Characteristic {
        uuid: runtime.write_uuid,
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: false,
            method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, req| {
                let command_events = command_events.clone();
                let write_router = write_router_for_write.clone();
                let write_notify_tx = write_notify_tx.clone();
                let link_diagnostics = link_diagnostics_for_write.clone();
                Box::pin(async move {
                    let diagnostics =
                        server::ble_diagnostics::WriteDiagnostics::from_raw(&new_value);
                    link_diagnostics.record_write(&diagnostics);
                    info!(
                        adapter_name = %req.adapter_name,
                        device_address = %req.device_address,
                        offset = req.offset,
                        mtu = req.mtu,
                        op_type = ?req.op_type,
                        link = ?req.link,
                        prepare_authorize = req.prepare_authorize,
                        payload_bytes = diagnostics.payload_bytes,
                        frame_kind = diagnostics.frame_kind,
                        stream_id = diagnostics.stream_id,
                        frame_index = diagnostics.frame_index,
                        likely_probe = diagnostics.likely_probe,
                        preview_hex = %diagnostics.preview_hex,
                        "ble.write.received"
                    );
                    let route_result = {
                        let mut router = write_router.lock().await;
                        router.accept_write(&new_value)
                    };
                    match route_result {
                        Ok(server::transport::WriteEvent::Request {
                            request: req,
                            transport,
                        }) => {
                            if let Some(transport) = transport.as_ref() {
                                server::gatt_write::schedule_transport_request_ack(
                                    write_notify_tx.clone(),
                                    transport.stream_id,
                                    transport.final_frame_index,
                                    diagnostics.likely_probe,
                                );
                            }
                            let command_name = req.payload.command_name().to_string();
                            info!(
                                request_id = %req.id,
                                cmd = %command_name,
                                protocol_version = %req.v,
                                payload_bytes = new_value.len(),
                                transport_stream_id = transport.as_ref().map(|context| context.stream_id),
                                "ble.request.received"
                            );

                            tokio::spawn(async move {
                                match req.payload.clone() {
                                    protocol::requests::CommandPayload::LinkAck(ack) => {
                                        command_events.handle_ack(ack, &req.id).await;
                                    }
                                    _ => {
                                        if let Some(transport) = transport {
                                            command_events
                                                .handle_transport_request(
                                                    req,
                                                    command_name,
                                                    transport,
                                                )
                                                .await;
                                        } else {
                                            command_events.handle_request(req, command_name).await;
                                        }
                                    }
                                }
                            });
                        }
                        Ok(server::transport::WriteEvent::TransportAck(ack)) => {
                            info!(
                                frame_kind = server::ble_diagnostics::frame_kind_name(ack.kind),
                                stream_id = ack.stream_id,
                                frame_index = ack.index,
                                likely_probe = diagnostics.likely_probe,
                                "ble.transport.ack.received"
                            );
                            tokio::spawn(async move {
                                command_events.handle_transport_ack(ack).await;
                            });
                        }
                        Ok(server::transport::WriteEvent::Incomplete { transport }) => {
                            info!(
                                frame_kind = diagnostics.frame_kind,
                                stream_id = diagnostics.stream_id,
                                frame_index = diagnostics.frame_index,
                                duplicate = transport.as_ref().map(|context| context.duplicate),
                                "ble.write.incomplete"
                            );
                            if let Some(transport) = transport {
                                server::gatt_write::schedule_transport_request_ack(
                                    write_notify_tx.clone(),
                                    transport.stream_id,
                                    transport.index,
                                    diagnostics.likely_probe,
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                payload_bytes = new_value.len(),
                                frame_kind = diagnostics.frame_kind,
                                stream_id = diagnostics.stream_id,
                                frame_index = diagnostics.frame_index,
                                likely_probe = diagnostics.likely_probe,
                                preview_hex = %diagnostics.preview_hex,
                                "ble.request.parse_failed"
                            );
                            if let Some(response) = parse_router_error_response(&new_value, &e) {
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
                let notify_session_seq = notify_session_seq.clone();
                let write_router = write_router_for_notify.clone();
                let link_diagnostics = link_diagnostics_for_notify.clone();
                async move {
                    tokio::spawn(async move {
                        let session_id = notify_session_seq.fetch_add(1, Ordering::Relaxed) + 1;
                        let confirming = notifier.confirming();
                        {
                            let mut router = write_router.lock().await;
                            router.reset();
                        }
                        link_diagnostics.reset();
                        info!(
                            notify_session_id = session_id,
                            confirming,
                            "ble.notify.subscribed"
                        );
                        info!(
                            notify_session_id = session_id,
                            "ble.transport.router.reset"
                        );
                        let stopped = notifier.stopped();
                        tokio::pin!(stopped);
                        loop {
                            tokio::select! {
                                _ = &mut stopped => {
                                    let snapshot = link_diagnostics.snapshot();
                                    warn!(
                                        notify_session_id = session_id,
                                        last_notify = ?snapshot,
                                        "ble.notify.stopped"
                                    );
                                    break;
                                }
                                received = rx.recv() => {
                                    match received {
                                        Ok(value) => {
                                            let preview_hex =
                                                server::ble_diagnostics::preview_hex(&value, 24);
                                            let snapshot = link_diagnostics.next_notify(&value);
                                            info!(
                                                notify_session_id = session_id,
                                                notify_seq = snapshot.notify_seq,
                                                bytes = value.len(),
                                                stopped = notifier.is_stopped(),
                                                frame_kind = snapshot.frame_kind,
                                                stream_id = snapshot.stream_id,
                                                frame_index = snapshot.frame_index,
                                                ms_since_prev_notify = snapshot.ms_since_prev_notify,
                                                last_ack_stream_id = snapshot.last_ack_stream_id,
                                                last_ack_index = snapshot.last_ack_index,
                                                ms_since_last_ack = snapshot.ms_since_last_ack,
                                                last_write_kind = snapshot.last_write_kind,
                                                last_write_stream_id = snapshot.last_write_stream_id,
                                                last_write_index = snapshot.last_write_index,
                                                ms_since_last_write = snapshot.ms_since_last_write,
                                                preview_hex = %preview_hex,
                                                "ble.notify.tx.start"
                                            );
                                            if let Err(err) = notifier.notify(value).await {
                                                let latest = link_diagnostics.snapshot();
                                                warn!(
                                                    notify_session_id = session_id,
                                                    notify_seq = snapshot.notify_seq,
                                                    error = %err,
                                                    last_notify = ?latest,
                                                    "ble.notify.failed"
                                                );
                                                break;
                                            }
                                            let finished = link_diagnostics.mark_notify_finished(snapshot.notify_seq);
                                            info!(
                                                notify_session_id = session_id,
                                                notify_seq = snapshot.notify_seq,
                                                last_notify = ?finished,
                                                "ble.notify.tx.done"
                                            );
                                        }
                                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                            warn!(
                                                notify_session_id = session_id,
                                                skipped,
                                                "ble.notify.lagged"
                                            );
                                        }
                                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                                    }
                                    if notifier.is_stopped() {
                                        let snapshot = link_diagnostics.snapshot();
                                        warn!(
                                            notify_session_id = session_id,
                                            last_notify = ?snapshot,
                                            "ble.notify.stopped"
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                        info!(
                            notify_session_id = session_id,
                            "ble.notify.closed"
                        );
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
        &runtime,
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

#[cfg(target_os = "linux")]
fn parse_router_error_response(
    raw: &[u8],
    err: &server::transport::WriteRouterError,
) -> Option<protocol::CommandResponse> {
    match err {
        server::transport::WriteRouterError::Protocol(err) => {
            protocol::parse_error_response(raw, err)
        }
        server::transport::WriteRouterError::Transport(_) => None,
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    // This empty main is compiled only when OS is not Linux,
    // so `cargo build` doesn't complain about missing `main` before hitting the `compile_error!`.
}
