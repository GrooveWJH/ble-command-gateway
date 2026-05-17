use std::env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvertisingBackend {
    BluezDbus,
    LegacyHci,
}

impl AdvertisingBackend {
    pub fn from_env() -> Self {
        match env::var("YUNDRONE_BLE_ADV_BACKEND") {
            Ok(value) if value.eq_ignore_ascii_case("legacy-hci") => Self::LegacyHci,
            _ => Self::BluezDbus,
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
        std::env::remove_var("YUNDRONE_BLE_ADV_BACKEND");

        assert_eq!(AdvertisingBackend::from_env(), AdvertisingBackend::BluezDbus);
    }

    #[test]
    fn accepts_legacy_hci_backend_from_env() {
        std::env::set_var("YUNDRONE_BLE_ADV_BACKEND", "legacy-hci");

        assert_eq!(AdvertisingBackend::from_env(), AdvertisingBackend::LegacyHci);

        std::env::remove_var("YUNDRONE_BLE_ADV_BACKEND");
    }
}
