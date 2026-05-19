#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub prefix: String,
    pub name: String,
}

pub fn build_device_identity(prefix: &str, name: String) -> DeviceIdentity {
    DeviceIdentity {
        prefix: prefix.to_string(),
        name,
    }
}

pub fn validate_name_prefix(prefix: &str) -> Result<(), String> {
    if prefix.is_empty() {
        return Err("device prefix must not be empty".to_string());
    }
    if prefix.starts_with('-') || prefix.ends_with('-') || prefix.contains("--") {
        return Err("device prefix must not start/end with '-' or contain '--'".to_string());
    }
    if !prefix
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(
            "device prefix must contain only lowercase ASCII letters, digits, and '-'".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_device_identity, validate_name_prefix};

    #[test]
    fn stores_single_identity_name() {
        let identity = build_device_identity("yundrone", "yundrone-ytcwln".to_string());

        assert_eq!(identity.prefix, "yundrone");
        assert_eq!(identity.name, "yundrone-ytcwln");
    }

    #[test]
    fn validates_lowercase_dash_prefixes() {
        assert!(validate_name_prefix("yundrone").is_ok());
        assert!(validate_name_prefix("my-drone1").is_ok());
        assert!(validate_name_prefix("").is_err());
        assert!(validate_name_prefix("Yundrone").is_err());
        assert!(validate_name_prefix("yun_drone").is_err());
        assert!(validate_name_prefix("-yundrone").is_err());
        assert!(validate_name_prefix("yundrone-").is_err());
        assert!(validate_name_prefix("yun--drone").is_err());
    }
}
