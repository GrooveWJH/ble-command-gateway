use std::env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvertisingBackend {
    Auto,
    BluezDbus,
    LegacyHci,
}

impl AdvertisingBackend {
    pub fn from_env() -> Self {
        Self::from_value(env::var("YUNDRONE_BLE_ADV_BACKEND").ok().as_deref())
    }

    pub fn from_value(value: Option<&str>) -> Self {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) if value.eq_ignore_ascii_case("bluez-dbus") => Self::BluezDbus,
            Some(value) if value.eq_ignore_ascii_case("legacy-hci") => Self::LegacyHci,
            _ => Self::Auto,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "bluez-dbus" => Ok(Self::BluezDbus),
            "legacy-hci" => Ok(Self::LegacyHci),
            _ => Err("广播后端必须是 auto、bluez-dbus 或 legacy-hci".to_string()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::BluezDbus => "bluez-dbus",
            Self::LegacyHci => "legacy-hci",
        }
    }

    pub fn select(
        requested: Self,
        capabilities: &crate::advertising::AdvertisingCapabilitiesSnapshot,
        bluetoothd: &crate::bluetoothd::BluetoothdEnvironment,
        legacy_available: bool,
    ) -> anyhow::Result<(Self, &'static str)> {
        match requested {
            Self::BluezDbus => Ok((Self::BluezDbus, "explicit configuration")),
            Self::LegacyHci => {
                if legacy_available {
                    Ok((Self::LegacyHci, "explicit configuration"))
                } else {
                    anyhow::bail!("legacy-hci requested but hcitool is unavailable")
                }
            }
            Self::Auto => {
                if !bluetoothd.has_experimental && legacy_available {
                    return Ok((
                        Self::LegacyHci,
                        "bluetoothd is not experimental and hcitool is available",
                    ));
                }
                if capabilities.supported_instances.is_some() {
                    return Ok((Self::BluezDbus, "BlueZ LEAdvertisingManager1 is available"));
                }
                if legacy_available {
                    return Ok((
                        Self::LegacyHci,
                        "BlueZ advertising capabilities unavailable",
                    ));
                }
                anyhow::bail!(
                    "no usable advertising backend: BlueZ LEAdvertisingManager1 unavailable, bluetoothd experimental={}, hcitool unavailable",
                    bluetoothd.has_experimental
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AdvertisingBackend;

    #[test]
    fn defaults_to_auto_backend() {
        assert_eq!(
            AdvertisingBackend::from_value(None),
            AdvertisingBackend::Auto
        );
    }

    #[test]
    fn accepts_legacy_hci_backend_from_value() {
        assert_eq!(
            AdvertisingBackend::from_value(Some("legacy-hci")),
            AdvertisingBackend::LegacyHci
        );
    }

    #[test]
    fn rejects_unknown_backend() {
        assert!(AdvertisingBackend::parse("invalid").is_err());
    }

    #[test]
    fn auto_prefers_legacy_without_experimental_when_hcitool_exists() {
        let caps = crate::advertising::AdvertisingCapabilitiesSnapshot {
            active_instances: Some(0),
            supported_instances: Some(4),
            max_advertisement_length: Some(31),
            max_scan_response_length: Some(31),
            max_tx_power: None,
            can_set_tx_power: false,
            secondary_channels: vec![],
            platform_features: vec![],
        };
        let env = crate::bluetoothd::BluetoothdEnvironment {
            has_experimental: false,
            command_line: None,
        };
        let (selected, _) =
            AdvertisingBackend::select(AdvertisingBackend::Auto, &caps, &env, true).unwrap();
        assert_eq!(selected, AdvertisingBackend::LegacyHci);
    }

    #[test]
    fn auto_uses_dbus_when_experimental_and_hcitool_missing() {
        let caps = crate::advertising::AdvertisingCapabilitiesSnapshot {
            active_instances: Some(0),
            supported_instances: Some(4),
            max_advertisement_length: Some(31),
            max_scan_response_length: Some(31),
            max_tx_power: None,
            can_set_tx_power: false,
            secondary_channels: vec![],
            platform_features: vec![],
        };
        let env = crate::bluetoothd::BluetoothdEnvironment {
            has_experimental: true,
            command_line: None,
        };
        let (selected, _) =
            AdvertisingBackend::select(AdvertisingBackend::Auto, &caps, &env, false).unwrap();
        assert_eq!(selected, AdvertisingBackend::BluezDbus);
    }
}
