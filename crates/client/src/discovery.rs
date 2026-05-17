use btleplug::api::PeripheralProperties;
use uuid::Uuid;

pub const SHORT_NAME_PREFIX: &str = "YD-";
pub const UART_SERVICE_UUID: &str = "6E400001-B5A3-F393-E0A9-E50E24DCCA9E";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryCriteria {
    pub stable_prefix: String,
    pub short_name_prefix: String,
    pub service_uuid: Uuid,
}

impl DiscoveryCriteria {
    pub fn for_prefix(stable_prefix: &str) -> Self {
        Self {
            stable_prefix: stable_prefix.to_string(),
            short_name_prefix: SHORT_NAME_PREFIX.to_string(),
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
    let has_uart_service = properties.services.contains(&criteria.service_uuid);
    let candidate_name = extract_candidate_name(raw_name, criteria);
    let matches_identity = has_uart_service
        && (candidate_name.is_some() || raw_name.starts_with(&criteria.short_name_prefix));

    Some(DiscoveryMatch {
        display_name: raw_name.to_string(),
        candidate_name,
        matches_identity,
    })
}

fn extract_candidate_name(raw_name: &str, criteria: &DiscoveryCriteria) -> Option<String> {
    if raw_name.starts_with(&criteria.stable_prefix) || raw_name.starts_with(&criteria.short_name_prefix)
    {
        return Some(raw_name.to_string());
    }

    let start = raw_name.find('[')?;
    let end = raw_name.rfind(']')?;
    if end <= start + 1 {
        return None;
    }

    let inner = raw_name[start + 1..end].trim();
    if inner.starts_with(&criteria.stable_prefix) || inner.starts_with(&criteria.short_name_prefix) {
        Some(inner.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_properties, DiscoveryCriteria, SHORT_NAME_PREFIX};
    use btleplug::api::PeripheralProperties;

    fn base_properties() -> PeripheralProperties {
        PeripheralProperties::default()
    }

    #[test]
    fn matches_short_name_when_uart_service_is_present() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("Yundrone_UAV");
        properties.local_name = Some("YD-A3FB".to_string());
        properties.services = vec![criteria.service_uuid];

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(matched.matches_identity);
        assert_eq!(matched.candidate_name.as_deref(), Some("YD-A3FB"));
    }

    #[test]
    fn matches_bracketed_full_name_when_uart_service_is_present() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("Yundrone_UAV");
        properties.local_name = Some("orangepi4pro [Yundrone_UAV-03-17-5433]".to_string());
        properties.services = vec![criteria.service_uuid];

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(matched.matches_identity);
        assert_eq!(
            matched.candidate_name.as_deref(),
            Some("Yundrone_UAV-03-17-5433")
        );
    }

    #[test]
    fn rejects_short_name_without_uart_service() {
        let mut properties = base_properties();
        let criteria = DiscoveryCriteria::for_prefix("Yundrone_UAV");
        properties.local_name = Some(format!("{SHORT_NAME_PREFIX}A3FB"));

        let matched = classify_properties(&properties, &criteria).unwrap();

        assert!(!matched.matches_identity);
        assert_eq!(matched.candidate_name.as_deref(), Some("YD-A3FB"));
    }
}
