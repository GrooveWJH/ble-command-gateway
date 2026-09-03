#[cfg(target_os = "linux")]
use bluer::{Adapter, Address, Session};
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};
#[cfg(target_os = "linux")]
use uuid::Uuid;

#[cfg(target_os = "linux")]
const DEFAULT_ADAPTER_WAIT_SECS: u64 = 60;
#[cfg(target_os = "linux")]
const ADAPTER_RETRY_INTERVAL: Duration = Duration::from_millis(500);
#[cfg(target_os = "linux")]
const ADAPTER_WAIT_LOG_INTERVAL: Duration = Duration::from_secs(5);

#[cfg(target_os = "linux")]
#[derive(Clone, Debug)]
pub struct ServerRuntimeContext {
    pub backend_preference: crate::advertising_backend::AdvertisingBackend,
    pub advertising_backend: crate::advertising_backend::AdvertisingBackend,
    pub fallback_attempted: bool,
    pub identity: crate::device_identity::DeviceIdentity,
    pub adapter_address: Address,
    pub advertising_policy: crate::advertising::AdvertisingPolicy,
    pub service_uuid: Uuid,
    pub write_uuid: Uuid,
    pub read_uuid: Uuid,
}

#[cfg(target_os = "linux")]
pub fn build_runtime_context(
    args: crate::config::ServerArgs,
    adapter_address: Address,
) -> anyhow::Result<ServerRuntimeContext> {
    let name_prefix = crate::config::device_prefix_from_args(&args)?;
    let identity_name =
        crate::device_name::build_device_name_from_mac(&name_prefix, Some(adapter_address.0));
    Ok(ServerRuntimeContext {
        backend_preference: args.backend,
        advertising_backend: args.backend,
        fallback_attempted: false,
        identity: crate::device_identity::build_device_identity(&name_prefix, identity_name),
        adapter_address,
        advertising_policy: crate::advertising::default_policy(),
        service_uuid: Uuid::parse_str("6E400001-B5A3-F393-E0A9-E50E24DCCA9E")?,
        write_uuid: Uuid::parse_str("6E400002-B5A3-F393-E0A9-E50E24DCCA9E")?,
        read_uuid: Uuid::parse_str("6E400003-B5A3-F393-E0A9-E50E24DCCA9E")?,
    })
}

#[cfg(target_os = "linux")]
pub async fn wait_for_default_adapter(
    session: &Session,
    name_prefix: &str,
) -> anyhow::Result<(Adapter, Address)> {
    wait_for_adapter(session, None, name_prefix).await
}

#[cfg(target_os = "linux")]
pub async fn wait_for_adapter(
    session: &Session,
    requested_name: Option<&str>,
    name_prefix: &str,
) -> anyhow::Result<(Adapter, Address)> {
    let timeout = adapter_wait_timeout();
    let started = Instant::now();
    let mut next_log = started;

    loop {
        let adapter_result = match requested_name {
            Some(name) => session
                .adapter(name)
                .map_err(|err| format!("adapter {name} unavailable: {err}")),
            None => session
                .default_adapter()
                .await
                .map_err(|err| format!("adapter unavailable: {err}")),
        };
        let last_error = match adapter_result {
            Ok(adapter) => match adapter.address().await {
                Ok(address) if crate::device_name::is_usable_mac_address(&address.0) => {
                    tracing::info!(
                        adapter_name = %adapter.name(),
                        adapter_address = %address,
                        requested_adapter = ?requested_name,
                        waited_ms = started.elapsed().as_millis(),
                        "ble.adapter.identity_ready"
                    );
                    return Ok((adapter, address));
                }
                Ok(address) => format!("adapter returned unusable address {address}"),
                Err(err) => format!("adapter address unavailable: {err}"),
            },
            Err(err) => err,
        };

        let elapsed = started.elapsed();
        if elapsed >= timeout {
            let identity_name = crate::device_name::build_device_name_from_mac(name_prefix, None);
            tracing::error!(
                identity_name = %identity_name,
                waited_secs = timeout.as_secs(),
                error = %last_error,
                "ble.adapter.wait_timeout"
            );
            anyhow::bail!(
                "Bluetooth adapter unavailable after {}s; derived identity is {identity_name}: {last_error}",
                timeout.as_secs()
            );
        }

        let now = Instant::now();
        if now >= next_log {
            tracing::warn!(
                identity_name = %crate::device_name::build_device_name_from_mac(name_prefix, None),
                waited_ms = elapsed.as_millis(),
                wait_limit_secs = timeout.as_secs(),
                error = %last_error,
                "ble.adapter.waiting"
            );
            next_log = now + ADAPTER_WAIT_LOG_INTERVAL;
        }

        tokio::time::sleep(ADAPTER_RETRY_INTERVAL.min(timeout.saturating_sub(elapsed))).await;
    }
}

#[cfg(target_os = "linux")]
fn adapter_wait_timeout() -> Duration {
    let seconds = std::env::var("YUNDRONE_BLE_ADAPTER_WAIT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_ADAPTER_WAIT_SECS);
    Duration::from_secs(seconds)
}

#[cfg(target_os = "linux")]
pub async fn log_advertising_environment(
    adapter: &Adapter,
    capabilities: &crate::advertising::AdvertisingCapabilitiesSnapshot,
    environment: &crate::bluetoothd::BluetoothdEnvironment,
    context: &ServerRuntimeContext,
) {
    let adapter_name = adapter.name();

    tracing::info!(
        adapter_address = %context.adapter_address,
        identity_name = %context.identity.name,
        identity_source = "adapter-mac",
        "ble.identity.derived"
    );

    tracing::info!(
        adapter_name = %adapter_name,
        identity_name = %context.identity.name,
        identity_prefix = %context.identity.prefix,
        adapter_address = %context.adapter_address,
        advertising_backend = context.advertising_backend.as_str(),
        "ble.server.starting"
    );
    crate::log_view::emit_block(&crate::log_view::startup_block(
        adapter_name,
        &context.identity.name,
        "adapter-mac",
        context.advertising_backend.as_str(),
    ));
    tracing::info!(
        adapter_name = %adapter_name,
        active_instances = ?capabilities.active_instances,
        supported_instances = ?capabilities.supported_instances,
        max_advertisement_length = ?capabilities.max_advertisement_length,
        max_scan_response_length = ?capabilities.max_scan_response_length,
        max_tx_power = ?capabilities.max_tx_power,
        can_set_tx_power = capabilities.can_set_tx_power,
        secondary_channels = ?capabilities.secondary_channels,
        platform_features = ?capabilities.platform_features,
        "ble.advertising.capabilities"
    );
    tracing::info!(
        adapter_name = %adapter_name,
        bluez_dbus_available = capabilities.supported_instances.is_some(),
        legacy_hci_available = crate::legacy_hci::is_available(),
        experimental = environment.has_experimental,
        bluetoothd = ?environment.command_line,
        "ble.adapter.capabilities"
    );
    tracing::warn!(
        adapter_name = %adapter_name,
        identity_name = %context.identity.name,
        payload_hint = %crate::advertising::payload_risk_hint(
            &context.identity.name,
            capabilities
        ),
        "ble.advertising.payload_risk"
    );
    if environment.has_experimental {
        tracing::info!(
            adapter_name = %adapter_name,
            bluetoothd = ?environment.command_line,
            "ble.advertising.experimental_ready"
        );
    } else {
        tracing::warn!(
            adapter_name = %adapter_name,
            bluetoothd = ?environment.command_line,
            "ble.advertising.experimental_required"
        );
    }
    tracing::warn!(adapter_name = %adapter_name, "ble.advertising.interval_unverified");
}

#[cfg(target_os = "linux")]
pub enum AdvertisingSession {
    Bluez(bluer::adv::AdvertisementHandle),
    Legacy(crate::legacy_hci::LegacyAdvertisingSession),
}

#[cfg(target_os = "linux")]
impl AdvertisingSession {
    pub async fn stop(self) -> anyhow::Result<()> {
        match self {
            Self::Bluez(handle) => {
                drop(handle);
                Ok(())
            }
            Self::Legacy(session) => session.stop().await,
        }
    }
}

#[cfg(target_os = "linux")]
pub async fn start_advertising(
    adapter: &Adapter,
    capabilities: &crate::advertising::AdvertisingCapabilitiesSnapshot,
    context: &mut ServerRuntimeContext,
    phase: crate::advertising::AdvertisingPhase,
) -> anyhow::Result<AdvertisingSession> {
    let config =
        crate::advertising::applied_config(&context.advertising_policy, phase, capabilities);
    let adapter_name = adapter.name();
    let session = match context.advertising_backend {
        crate::advertising_backend::AdvertisingBackend::BluezDbus => {
            match crate::advertising::advertise_phase(
                adapter,
                &context.identity.name,
                context.service_uuid,
                config,
            )
            .await
            {
                Ok(handle) => AdvertisingSession::Bluez(handle),
                Err(err)
                    if matches!(
                        context.backend_preference,
                        crate::advertising_backend::AdvertisingBackend::Auto
                    ) && !context.fallback_attempted
                        && crate::legacy_hci::is_available() =>
                {
                    context.fallback_attempted = true;
                    tracing::warn!(
                        adapter_name = %adapter_name,
                        from = "bluez-dbus",
                        to = "legacy-hci",
                        error = %err,
                        "ble.advertising.fallback"
                    );
                    context.advertising_backend =
                        crate::advertising_backend::AdvertisingBackend::LegacyHci;
                    AdvertisingSession::Legacy(
                        crate::legacy_hci::start_legacy_advertising(
                            adapter_name,
                            &context.identity.name,
                            context.service_uuid,
                            config.interval,
                        )
                        .await?,
                    )
                }
                Err(err) => {
                    return Err(anyhow::anyhow!(
                        "advertising backend {} failed: {}",
                        context.advertising_backend.as_str(),
                        err
                    ))
                }
            }
        }
        crate::advertising_backend::AdvertisingBackend::LegacyHci => AdvertisingSession::Legacy(
            crate::legacy_hci::start_legacy_advertising(
                adapter_name,
                &context.identity.name,
                context.service_uuid,
                config.interval,
            )
            .await?,
        ),
        crate::advertising_backend::AdvertisingBackend::Auto => {
            return Err(anyhow::anyhow!(
                "advertising backend was not selected before start"
            ));
        }
    };

    if matches!(phase, crate::advertising::AdvertisingPhase::FastStart) {
        tracing::info!(
            adapter_name = %adapter_name,
            identity_name = %context.identity.name,
            advertising_backend = context.advertising_backend.as_str(),
            phase = crate::advertising::phase_name(config.phase),
            fast_duration_secs = context.advertising_policy.fast_duration.as_secs(),
            "ble.advertising.fast_start"
        );
    } else {
        tracing::info!(
            adapter_name = %adapter_name,
            identity_name = %context.identity.name,
            advertising_backend = context.advertising_backend.as_str(),
            phase = crate::advertising::phase_name(config.phase),
            min_interval = %crate::advertising::interval_ms_text(config.interval.min),
            max_interval = %crate::advertising::interval_ms_text(config.interval.max),
            tx_power = ?config.tx_power,
            "ble.advertising.reset_to_steady"
        );
    }

    tracing::info!(
        adapter_name = %adapter_name,
        identity_name = %context.identity.name,
        advertising_backend = context.advertising_backend.as_str(),
        phase = crate::advertising::phase_name(config.phase),
        min_interval = %crate::advertising::interval_ms_text(config.interval.min),
        max_interval = %crate::advertising::interval_ms_text(config.interval.max),
        tx_power = ?config.tx_power,
        "ble.advertising.ready"
    );
    crate::log_view::emit_block(&crate::log_view::advertising_block(
        &crate::log_view::AdvertisingLogView {
            title: "BLE advertising",
            adapter_name,
            identity_name: &context.identity.name,
            phase: crate::advertising::phase_name(config.phase),
            min_interval: &crate::advertising::interval_ms_text(config.interval.min),
            max_interval: &crate::advertising::interval_ms_text(config.interval.max),
            tx_power: &format!("{:?}", config.tx_power),
        },
    ));
    tracing::info!(
        adapter_name = %adapter_name,
        identity_name = %context.identity.name,
        advertising_backend = context.advertising_backend.as_str(),
        phase = crate::advertising::phase_name(config.phase),
        min_interval = %crate::advertising::interval_ms_text(config.interval.min),
        max_interval = %crate::advertising::interval_ms_text(config.interval.max),
        tx_power = ?config.tx_power,
        "ble.advertising.config_applied"
    );

    Ok(session)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn runtime_context_uses_adapter_mac_identity() {
        let address = bluer::Address::new([0xdc, 0xa6, 0x32, 0x12, 0xab, 0xcd]);
        let context = super::build_runtime_context(
            crate::config::ServerArgs {
                name_prefix: "yundrone".to_string(),
                adapter: None,
                backend: crate::advertising_backend::AdvertisingBackend::Auto,
            },
            address,
        )
        .unwrap();

        assert_eq!(context.identity.name, "yundrone-12abcd");
        assert_eq!(context.adapter_address, address);
    }
}
