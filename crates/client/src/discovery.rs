use btleplug::api::PeripheralProperties;
use uuid::Uuid;

pub const UART_SERVICE_UUID: &str = "6E400001-B5A3-F393-E0A9-E50E24DCCA9E";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryCriteria {
    pub stable_prefix: String,
    pub service_uuid: Uuid,
}

impl DiscoveryCriteria {
    pub fn for_prefix(stable_prefix: &str) -> Self {
        Self {
            stable_prefix: stable_prefix.to_string(),
            service_uuid: Uuid::parse_str(UART_SERVICE_UUID).expect("valid UART service UUID"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryMatch {
    pub display_name: String,
    pub candidate_name: Option<String>,
    pub matches_identity: bool,
}

pub fn classify_properties(
    properties: &PeripheralProperties,
    criteria: &DiscoveryCriteria,
) -> Option<DiscoveryMatch> {
    let raw_name = properties.local_name.as_deref()?;
    let candidate_name = extract_candidate_name(raw_name, criteria);
    let matches_identity = candidate_name.is_some();

    Some(DiscoveryMatch {
        display_name: raw_name.to_string(),
        candidate_name,
        matches_identity,
    })
}

fn extract_candidate_name(raw_name: &str, criteria: &DiscoveryCriteria) -> Option<String> {
    if is_stable_identity(raw_name, &criteria.stable_prefix) {
        return Some(raw_name.to_string());
    }

    let start = raw_name.find('[')?;
    let end = raw_name.rfind(']')?;
    if end <= start + 1 {
        return None;
    }

    let inner = raw_name[start + 1..end].trim();
    if is_stable_identity(inner, &criteria.stable_prefix) {
        Some(inner.to_string())
    } else {
        None
    }
}

fn is_stable_identity(value: &str, prefix: &str) -> bool {
    let Some(suffix) = value
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('-'))
    else {
        return false;
    };
    suffix.len() == 6
        && suffix
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{classify_properties, DiscoveryCriteria};
    use btleplug::api::PeripheralProperties;

    fn base_properties() -> PeripheralProperties {
        PeripheralProperties::default()
    }

    #[test]
    fn matches_configured_prefix_when_uart_service_is_present() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("yundrone-ytcwln".to_string());
        properties.services = vec![criteria.service_uuid];

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(matched.matches_identity);
        assert_eq!(matched.candidate_name.as_deref(), Some("yundrone-ytcwln"));
    }

    #[test]
    fn matches_bracketed_full_name_when_uart_service_is_present() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("edge-gateway [yundrone-ytcwln]".to_string());
        properties.services = vec![criteria.service_uuid];

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(matched.matches_identity);
        assert_eq!(matched.candidate_name.as_deref(), Some("yundrone-ytcwln"));
    }

    #[test]
    fn accepts_configured_prefix_without_uart_service() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("yundrone-ytcwln".to_string());

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(matched.matches_identity);
        assert_eq!(matched.candidate_name.as_deref(), Some("yundrone-ytcwln"));
    }

    #[test]
    fn accepts_bracketed_full_name_without_uart_service() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("edge-gateway [yundrone-ytcwln]".to_string());

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(matched.matches_identity);
        assert_eq!(matched.candidate_name.as_deref(), Some("yundrone-ytcwln"));
    }

    #[test]
    fn rejects_stale_time_based_identity_from_cache() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("edge-gateway [yundrone-07-44-5433]".to_string());
        properties.services = vec![criteria.service_uuid];

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(!matched.matches_identity);
        assert!(matched.candidate_name.is_none());
    }

    #[test]
    fn rejects_uppercase_or_short_suffixes() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("yundrone-YTCWLN".to_string());

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(!matched.matches_identity);
        assert!(matched.candidate_name.is_none());
    }

    #[test]
    fn rejects_uart_service_without_configured_prefix() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("Gateway".to_string());
        properties.services = vec![criteria.service_uuid];

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(!matched.matches_identity);
        assert!(matched.candidate_name.is_none());
    }

    #[test]
    fn rejects_legacy_yundrone_uav_name() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("Yundrone_UAV-03-17-5433".to_string());

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(!matched.matches_identity);
        assert!(matched.candidate_name.is_none());
    }

    #[test]
    fn rejects_legacy_short_name() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("YD-A3FB".to_string());

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(!matched.matches_identity);
        assert!(matched.candidate_name.is_none());
    }

    #[test]
    fn keeps_non_matching_named_devices_for_raw_scan_logs() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");
        properties.local_name = Some("GrooveiPhone".to_string());

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert_eq!(matched.display_name, "GrooveiPhone");
        assert!(!matched.matches_identity);
        assert!(matched.candidate_name.is_none());
    }

    #[test]
    fn ignores_unnamed_devices() {
        let properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("yundrone");

        assert!(classify_properties(&properties, &criteria).is_none());
    }
}
