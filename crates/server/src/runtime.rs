#[cfg(target_os = "linux")]
use bluer::Adapter;
#[cfg(target_os = "linux")]
use uuid::Uuid;

#[cfg(target_os = "linux")]
#[derive(Clone, Debug)]
pub struct ServerRuntimeContext {
    pub advertising_backend: crate::advertising_backend::AdvertisingBackend,
    pub identity: crate::device_identity::DeviceIdentity,
    pub identity_source: crate::device_name::DeviceNameSource,
    pub advertising_policy: crate::advertising::AdvertisingPolicy,
    pub service_uuid: Uuid,
    pub write_uuid: Uuid,
    pub read_uuid: Uuid,
}

#[cfg(target_os = "linux")]
pub fn build_runtime_context(
    args: crate::config::ServerArgs,
) -> anyhow::Result<ServerRuntimeContext> {
    build_runtime_context_with_identity_path(
        args,
        std::path::Path::new(crate::device_name::DEFAULT_DEVICE_NAME_PATH),
    )
}

#[cfg(target_os = "linux")]
fn build_runtime_context_with_identity_path(
    args: crate::config::ServerArgs,
    identity_path: &std::path::Path,
) -> anyhow::Result<ServerRuntimeContext> {
    let name_prefix = crate::config::device_prefix_from_args(&args)?;
    let resolved_name =
        crate::device_name::resolve_persisted_device_name(&name_prefix, identity_path).map_err(
            |err| {
                tracing::error!(
                    path = %identity_path.display(),
                    error = %err,
                    "ble.identity.persist_failed"
                );
                anyhow::Error::new(err)
            },
        )?;
    Ok(ServerRuntimeContext {
        advertising_backend: crate::advertising_backend::AdvertisingBackend::from_env(),
        identity: crate::device_identity::build_device_identity(&name_prefix, resolved_name.name),
        identity_source: resolved_name.source,
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

    match &context.identity_source {
        crate::device_name::DeviceNameSource::File => {
            tracing::info!(
                path = crate::device_name::DEFAULT_DEVICE_NAME_PATH,
                identity_name = %context.identity.name,
                "ble.identity.loaded"
            );
        }
        crate::device_name::DeviceNameSource::Generated { reason } => {
            tracing::warn!(
                path = crate::device_name::DEFAULT_DEVICE_NAME_PATH,
                identity_name = %context.identity.name,
                reason = %reason,
                "ble.identity.recreated"
            );
        }
    }

    tracing::info!(
        adapter_name = %adapter_name,
        identity_name = %context.identity.name,
        identity_prefix = %context.identity.prefix,
        advertising_backend = context.advertising_backend.as_str(),
        "ble.server.starting"
    );
    crate::log_view::emit_block(&crate::log_view::startup_block(
        &adapter_name,
        &context.identity.name,
        context.identity_source.as_str(),
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
    context: &ServerRuntimeContext,
    phase: crate::advertising::AdvertisingPhase,
) -> anyhow::Result<AdvertisingSession> {
    let config =
        crate::advertising::applied_config(&context.advertising_policy, phase, capabilities);
    let adapter_name = adapter.name();
    let session = match context.advertising_backend {
        crate::advertising_backend::AdvertisingBackend::BluezDbus => AdvertisingSession::Bluez(
            crate::advertising::advertise_phase(
                adapter,
                &context.identity.name,
                context.service_uuid,
                config,
            )
            .await?,
        ),
        crate::advertising_backend::AdvertisingBackend::LegacyHci => AdvertisingSession::Legacy(
            crate::legacy_hci::start_legacy_advertising(
                &adapter_name,
                &context.identity.name,
                context.service_uuid,
                config.interval,
            )
            .await?,
        ),
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
            adapter_name: &adapter_name,
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
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "yundrone-runtime-{name}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn identity_file(&self) -> PathBuf {
            self.path.join("ble-device-name")
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn runtime_context_uses_persisted_identity_file() {
        let dir = TestDir::new("context");
        fs::write(dir.identity_file(), "yundrone-bw0uwj\n").unwrap();

        let context = super::build_runtime_context_with_identity_path(
            crate::config::ServerArgs {
                name_prefix: "yundrone".to_string(),
            },
            &dir.identity_file(),
        )
        .unwrap();

        assert_eq!(context.identity.name, "yundrone-bw0uwj");
        assert_eq!(
            context.identity_source,
            crate::device_name::DeviceNameSource::File
        );
    }
}
