use std::env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvertisingBackend {
    BluezDbus,
    LegacyHci,
}

impl AdvertisingBackend {
    pub fn from_env() -> Self {
        Self::from_value(env::var("YUNDRONE_BLE_ADV_BACKEND").ok().as_deref())
    }

    fn from_value(value: Option<&str>) -> Self {
        if matches!(value, Some(value) if value.eq_ignore_ascii_case("legacy-hci")) {
            Self::LegacyHci
        } else {
            Self::BluezDbus
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::BluezDbus => "bluez-dbus",
            Self::LegacyHci => "legacy-hci",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AdvertisingBackend;

    #[test]
    fn defaults_to_bluez_dbus_backend() {
        assert_eq!(
            AdvertisingBackend::from_value(None),
            AdvertisingBackend::BluezDbus
        );
    }

    #[test]
    fn accepts_legacy_hci_backend_from_value() {
        assert_eq!(
            AdvertisingBackend::from_value(Some("legacy-hci")),
            AdvertisingBackend::LegacyHci
        );
    }
}
