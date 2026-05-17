#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub stable_prefix: String,
    pub advertised_name: String,
    pub short_name: String,
}

pub fn build_device_identity(base_prefix: &str, advertised_name: String) -> DeviceIdentity {
    let short_name = short_name_from_advertised_name(&advertised_name)
        .unwrap_or_else(|| fallback_short_name(base_prefix));

    DeviceIdentity {
        stable_prefix: base_prefix.to_string(),
        advertised_name,
        short_name,
    }
}

pub fn short_name_from_advertised_name(advertised_name: &str) -> Option<String> {
    let suffix = advertised_name.rsplit('-').next()?.trim();
    if suffix.is_empty() || suffix == advertised_name {
        return None;
    }

    Some(format!("YD-{}", suffix.to_ascii_uppercase()))
}

fn fallback_short_name(base_prefix: &str) -> String {
    let mut initials = base_prefix
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .take(2)
        .filter_map(|part| part.chars().next())
        .map(|ch| ch.to_ascii_uppercase())
        .collect::<String>();

    if initials.is_empty() {
        initials.push_str("YD");
    }
    if initials.len() == 1 {
        initials.push('D');
    }
    format!("{initials}-UNK")
}

#[cfg(test)]
mod tests {
    use super::{build_device_identity, short_name_from_advertised_name};

    #[test]
    fn derives_short_name_from_full_instance_name() {
        let short_name = short_name_from_advertised_name("Yundrone_UAV-15-19-A3FB");

        assert_eq!(short_name.as_deref(), Some("YD-A3FB"));
    }

    #[test]
    fn falls_back_when_instance_name_has_no_suffix() {
        let identity = build_device_identity("Yundrone_UAV", "Yundrone_UAV".to_string());

        assert_eq!(identity.short_name, "YU-UNK");
    }
}
