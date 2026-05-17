#[cfg(target_os = "linux")]
use bluer::Adapter;
#[cfg(target_os = "linux")]
use uuid::Uuid;

#[cfg(target_os = "linux")]
#[derive(Clone, Debug)]
pub struct ServerRuntimeContext {
    pub advertising_backend: crate::advertising_backend::AdvertisingBackend,
    pub identity: crate::device_identity::DeviceIdentity,
    pub advertising_policy: crate::advertising::AdvertisingPolicy,
    pub service_uuid: Uuid,
    pub write_uuid: Uuid,
    pub read_uuid: Uuid,
}

#[cfg(target_os = "linux")]
pub fn build_runtime_context() -> anyhow::Result<ServerRuntimeContext> {
    Ok(ServerRuntimeContext {
        advertising_backend: crate::advertising_backend::AdvertisingBackend::from_env(),
        identity: crate::device_identity::build_device_identity(
            protocol::config::DEFAULT_DEVICE_NAME,
            crate::device_name::generate_device_name(protocol::config::DEFAULT_DEVICE_NAME),
        ),
        advertising_policy: crate::advertising::default_policy(),
        service_uuid: Uuid::parse_str("6E400001-B5A3-F393-E0A9-E50E24DCCA9E")?,
        write_uuid: Uuid::parse_str("6E400002-B5A3-F393-E0A9-E50E24DCCA9E")?,
        read_uuid: Uuid::parse_str("6E400003-B5A3-F393-E0A9-E50E24DCCA9E")?,
    })
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
        adapter_name = %adapter_name,
        advertised_name = %context.identity.advertised_name,
        short_name = %context.identity.short_name,
        advertising_backend = context.advertising_backend.as_str(),
        "ble.server.starting"
    );
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
    tracing::warn!(
        adapter_name = %adapter_name,
        advertised_name = %context.identity.advertised_name,
        short_name = %context.identity.short_name,
        payload_hint = %crate::advertising::payload_risk_hint(
            &context.identity.advertised_name,
            &context.identity.short_name,
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
    context: &ServerRuntimeContext,
    phase: crate::advertising::AdvertisingPhase,
) -> anyhow::Result<AdvertisingSession> {
    let config = crate::advertising::applied_config(&context.advertising_policy, phase, capabilities);
    let adapter_name = adapter.name();
    let session = match context.advertising_backend {
        crate::advertising_backend::AdvertisingBackend::BluezDbus => AdvertisingSession::Bluez(
            crate::advertising::advertise_phase(
                adapter,
                &context.identity.short_name,
                context.service_uuid,
                config,
            )
            .await?,
        ),
        crate::advertising_backend::AdvertisingBackend::LegacyHci => AdvertisingSession::Legacy(
            crate::legacy_hci::start_legacy_advertising(
                &adapter_name,
                &context.identity.short_name,
                &context.identity.advertised_name,
                context.service_uuid,
                config.interval,
            )
            .await?,
        ),
    };

    if matches!(phase, crate::advertising::AdvertisingPhase::FastStart) {
        tracing::info!(
            adapter_name = %adapter_name,
            advertised_name = %context.identity.advertised_name,
            short_name = %context.identity.short_name,
            advertising_backend = context.advertising_backend.as_str(),
            phase = crate::advertising::phase_name(config.phase),
            fast_duration_secs = context.advertising_policy.fast_duration.as_secs(),
            "ble.advertising.fast_start"
        );
    } else {
        tracing::info!(
            adapter_name = %adapter_name,
            advertised_name = %context.identity.advertised_name,
            short_name = %context.identity.short_name,
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
        advertised_name = %context.identity.advertised_name,
        short_name = %context.identity.short_name,
        advertising_backend = context.advertising_backend.as_str(),
        phase = crate::advertising::phase_name(config.phase),
        min_interval = %crate::advertising::interval_ms_text(config.interval.min),
        max_interval = %crate::advertising::interval_ms_text(config.interval.max),
        tx_power = ?config.tx_power,
        "ble.advertising.ready"
    );
    tracing::info!(
        adapter_name = %adapter_name,
        advertised_name = %context.identity.advertised_name,
        short_name = %context.identity.short_name,
        advertising_backend = context.advertising_backend.as_str(),
        phase = crate::advertising::phase_name(config.phase),
        min_interval = %crate::advertising::interval_ms_text(config.interval.min),
        max_interval = %crate::advertising::interval_ms_text(config.interval.max),
        tx_power = ?config.tx_power,
        "ble.advertising.config_applied"
    );

    Ok(session)
}
