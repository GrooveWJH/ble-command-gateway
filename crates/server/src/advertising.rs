use std::time::Duration;

#[cfg(target_os = "linux")]
use bluer::{
    adv::{Advertisement, AdvertisementHandle, PlatformFeature},
    Adapter,
};
#[cfg(target_os = "linux")]
use std::collections::BTreeSet;
#[cfg(target_os = "linux")]
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvertisingPhase {
    FastStart,
    Steady,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdvertisingInterval {
    pub min: Duration,
    pub max: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvertisingCapabilitiesSnapshot {
    pub active_instances: Option<u8>,
    pub supported_instances: Option<u8>,
    pub max_advertisement_length: Option<u8>,
    pub max_scan_response_length: Option<u8>,
    pub max_tx_power: Option<i16>,
    pub can_set_tx_power: bool,
    pub secondary_channels: Vec<String>,
    pub platform_features: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdvertisingPolicy {
    pub fast_interval: AdvertisingInterval,
    pub fast_duration: Duration,
    pub steady_interval: AdvertisingInterval,
    pub discoverable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppliedAdvertisingConfig {
    pub phase: AdvertisingPhase,
    pub interval: AdvertisingInterval,
    pub discoverable: bool,
    pub tx_power: Option<i16>,
}

pub fn default_policy() -> AdvertisingPolicy {
    AdvertisingPolicy {
        fast_interval: AdvertisingInterval {
            min: Duration::from_millis(20),
            max: Duration::from_millis(20),
        },
        fast_duration: Duration::from_secs(120),
        steady_interval: AdvertisingInterval {
            min: Duration::from_micros(152_500),
            max: Duration::from_micros(152_500),
        },
        discoverable: true,
    }
}

pub fn applied_config(
    policy: &AdvertisingPolicy,
    phase: AdvertisingPhase,
    capabilities: &AdvertisingCapabilitiesSnapshot,
) -> AppliedAdvertisingConfig {
    AppliedAdvertisingConfig {
        phase,
        interval: match phase {
            AdvertisingPhase::FastStart => policy.fast_interval,
            AdvertisingPhase::Steady => policy.steady_interval,
        },
        discoverable: policy.discoverable,
        tx_power: capabilities
            .can_set_tx_power
            .then_some(capabilities.max_tx_power)
            .flatten(),
    }
}

pub fn payload_risk_hint(
    advertised_name: &str,
    capabilities: &AdvertisingCapabilitiesSnapshot,
) -> String {
    format!(
        "device_name_len={} max_adv_len={:?} max_scan_rsp_len={:?}; local_name may be truncated or shifted to scan response, do not rely on scan response for identity",
        advertised_name.len(),
        capabilities.max_advertisement_length,
        capabilities.max_scan_response_length
    )
}

pub fn phase_name(phase: AdvertisingPhase) -> &'static str {
    match phase {
        AdvertisingPhase::FastStart => "fast_start",
        AdvertisingPhase::Steady => "steady",
    }
}

pub fn interval_ms_text(value: Duration) -> String {
    let millis = value.as_micros() as f64 / 1000.0;
    if millis.fract() == 0.0 {
        format!("{millis:.0} ms")
    } else {
        format!("{millis:.1} ms")
    }
}

#[cfg(target_os = "linux")]
pub async fn probe_capabilities(adapter: &Adapter) -> AdvertisingCapabilitiesSnapshot {
    let features = adapter
        .supported_advertising_features()
        .await
        .ok()
        .flatten()
        .map(|values| values.into_iter().map(|value| value.to_string()).collect())
        .unwrap_or_default();
    let secondary_channels = adapter
        .supported_advertising_secondary_channels()
        .await
        .ok()
        .flatten()
        .map(|values| values.into_iter().map(|value| value.to_string()).collect())
        .unwrap_or_default();
    let capabilities = adapter
        .supported_advertising_capabilities()
        .await
        .ok()
        .flatten();

    AdvertisingCapabilitiesSnapshot {
        active_instances: adapter.active_advertising_instances().await.ok(),
        supported_instances: adapter.supported_advertising_instances().await.ok(),
        max_advertisement_length: capabilities
            .as_ref()
            .map(|value| value.max_advertisement_length),
        max_scan_response_length: capabilities
            .as_ref()
            .map(|value| value.max_scan_response_length),
        max_tx_power: capabilities.as_ref().map(|value| value.max_tx_power),
        can_set_tx_power: adapter
            .supported_advertising_features()
            .await
            .ok()
            .flatten()
            .map(|values| values.contains(&PlatformFeature::CanSetTxPower))
            .unwrap_or(false),
        secondary_channels,
        platform_features: features,
    }
}

#[cfg(target_os = "linux")]
pub fn build_advertisement(
    advertised_name: &str,
    service_uuid: Uuid,
    config: AppliedAdvertisingConfig,
) -> Advertisement {
    Advertisement {
        advertisement_type: bluer::adv::Type::Peripheral,
        discoverable: Some(config.discoverable),
        local_name: Some(advertised_name.to_string()),
        service_uuids: [service_uuid].into_iter().collect::<BTreeSet<_>>(),
        min_interval: Some(config.interval.min),
        max_interval: Some(config.interval.max),
        tx_power: config.tx_power,
        ..Default::default()
    }
}

#[cfg(target_os = "linux")]
pub async fn advertise_phase(
    adapter: &Adapter,
    advertised_name: &str,
    service_uuid: Uuid,
    config: AppliedAdvertisingConfig,
) -> bluer::Result<AdvertisementHandle> {
    adapter
        .advertise(build_advertisement(advertised_name, service_uuid, config))
        .await
}

#[cfg(test)]
mod tests {
    use super::{
        AdvertisingCapabilitiesSnapshot, AdvertisingInterval, AdvertisingPhase, AdvertisingPolicy,
        AppliedAdvertisingConfig,
    };
    use std::time::Duration;

    #[test]
    fn fast_phase_uses_twenty_millisecond_interval() {
        let policy = AdvertisingPolicy {
            fast_interval: AdvertisingInterval {
                min: Duration::from_millis(20),
                max: Duration::from_millis(20),
            },
            fast_duration: Duration::from_secs(120),
            steady_interval: AdvertisingInterval {
                min: Duration::from_micros(152_500),
                max: Duration::from_micros(152_500),
            },
            discoverable: true,
        };
        let caps = AdvertisingCapabilitiesSnapshot {
            active_instances: Some(0),
            supported_instances: Some(4),
            max_advertisement_length: Some(31),
            max_scan_response_length: Some(31),
            max_tx_power: Some(8),
            can_set_tx_power: true,
            secondary_channels: vec![],
            platform_features: vec!["CanSetTxPower".to_string()],
        };

        assert_eq!(
            super::applied_config(&policy, AdvertisingPhase::FastStart, &caps),
            AppliedAdvertisingConfig {
                phase: AdvertisingPhase::FastStart,
                interval: policy.fast_interval,
                discoverable: true,
                tx_power: Some(8),
            }
        );
    }

    #[test]
    fn steady_phase_drops_tx_power_when_unsupported() {
        let policy = AdvertisingPolicy {
            fast_interval: AdvertisingInterval {
                min: Duration::from_millis(20),
                max: Duration::from_millis(20),
            },
            fast_duration: Duration::from_secs(120),
            steady_interval: AdvertisingInterval {
                min: Duration::from_micros(152_500),
                max: Duration::from_micros(152_500),
            },
            discoverable: true,
        };
        let caps = AdvertisingCapabilitiesSnapshot {
            active_instances: Some(1),
            supported_instances: Some(4),
            max_advertisement_length: Some(31),
            max_scan_response_length: Some(31),
            max_tx_power: Some(10),
            can_set_tx_power: false,
            secondary_channels: vec!["1M".to_string(), "2M".to_string()],
            platform_features: vec![],
        };

        assert_eq!(
            super::applied_config(&policy, AdvertisingPhase::Steady, &caps),
            AppliedAdvertisingConfig {
                phase: AdvertisingPhase::Steady,
                interval: policy.steady_interval,
                discoverable: true,
                tx_power: None,
            }
        );
    }

    #[test]
    fn payload_risk_hint_surfaces_controller_lengths() {
        let caps = AdvertisingCapabilitiesSnapshot {
            active_instances: Some(1),
            supported_instances: Some(4),
            max_advertisement_length: Some(31),
            max_scan_response_length: Some(31),
            max_tx_power: None,
            can_set_tx_power: false,
            secondary_channels: vec![],
            platform_features: vec![],
        };

        let hint = super::payload_risk_hint("Yundrone_UAV-14-20-5433", &caps);

        assert!(hint.contains("device_name_len=23"));
        assert!(hint.contains("max_adv_len=Some(31)"));
        assert!(hint.contains("scan response"));
    }

    #[test]
    fn interval_text_keeps_half_millisecond_precision() {
        assert_eq!(
            super::interval_ms_text(Duration::from_micros(152_500)),
            "152.5 ms"
        );
    }
}
