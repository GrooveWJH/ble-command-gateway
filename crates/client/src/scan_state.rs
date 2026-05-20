use std::collections::HashSet;
use std::hash::Hash;

use btleplug::platform::Peripheral;

use crate::ble::{progress_event, ScanCandidateInfo, ScanProgressEvent, ScannedDevice};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanObservation {
    pub progress: Option<ScanProgressEvent>,
    pub candidate_name: Option<String>,
    pub matches_identity: bool,
}

pub struct LiveScanState<K> {
    named_peripherals: HashSet<K>,
    matched_peripherals: HashSet<K>,
    pub named_device_count: usize,
    pub candidate_count: usize,
}

impl<K> Default for LiveScanState<K>
where
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> LiveScanState<K>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            named_peripherals: HashSet::new(),
            matched_peripherals: HashSet::new(),
            named_device_count: 0,
            candidate_count: 0,
        }
    }

    pub fn observe(
        &mut self,
        peripheral_id: K,
        display_name: &str,
        candidate_name: Option<String>,
        matches_identity: bool,
        rssi: Option<i16>,
    ) -> ScanObservation {
        let progress = if mark_peripheral_seen(&mut self.named_peripherals, peripheral_id.clone()) {
            self.named_device_count += 1;
            Some(progress_event(
                display_name.to_string(),
                rssi,
                matches_identity,
            ))
        } else {
            None
        };

        let candidate_name = candidate_name.filter(|_| {
            matches_identity && mark_peripheral_seen(&mut self.matched_peripherals, peripheral_id)
        });
        if candidate_name.is_some() {
            self.candidate_count += 1;
        }

        ScanObservation {
            progress,
            candidate_name,
            matches_identity,
        }
    }
}

pub fn build_scanned_device(
    peripheral: Peripheral,
    candidate_name: String,
    rssi: Option<i16>,
) -> ScannedDevice {
    ScannedDevice {
        info: ScanCandidateInfo {
            name: candidate_name,
            rssi,
        },
        peripheral,
    }
}

pub fn merge_scanned_device(existing: ScannedDevice, incoming: ScannedDevice) -> ScannedDevice {
    ScannedDevice {
        info: merge_candidate_info(existing.info, incoming.info),
        peripheral: incoming.peripheral,
    }
}

fn merge_candidate_info(
    existing: ScanCandidateInfo,
    incoming: ScanCandidateInfo,
) -> ScanCandidateInfo {
    let rssi = match (existing.rssi, incoming.rssi) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    };
    ScanCandidateInfo {
        name: incoming.name,
        rssi,
    }
}

fn mark_peripheral_seen<K>(seen: &mut HashSet<K>, peripheral_id: K) -> bool
where
    K: Eq + Hash,
{
    seen.insert(peripheral_id)
}

#[cfg(test)]
mod tests {
    use super::LiveScanState;
    use crate::ble::ScanCandidateInfo;

    #[test]
    fn observe_counts_named_and_matched_devices_once() {
        let mut state = LiveScanState::new();

        let first = state.observe(
            "dev-1".to_string(),
            "yundrone-ytcwln",
            Some("yundrone-ytcwln".to_string()),
            true,
            Some(-11),
        );
        let second = state.observe(
            "dev-1".to_string(),
            "yundrone-ytcwln",
            Some("yundrone-ytcwln".to_string()),
            true,
            Some(-10),
        );

        assert!(first.progress.is_some());
        assert!(first.candidate_name.is_some());
        assert!(second.progress.is_none());
        assert!(second.candidate_name.is_none());
        assert_eq!(state.named_device_count, 1);
        assert_eq!(state.candidate_count, 1);
    }

    #[test]
    fn candidate_merge_preserves_known_rssi_when_update_is_unknown() {
        let existing = ScanCandidateInfo {
            name: "yundrone-bw0uwj".to_string(),
            rssi: Some(-31),
        };
        let incoming = ScanCandidateInfo {
            name: "yundrone-bw0uwj".to_string(),
            rssi: None,
        };

        let merged = super::merge_candidate_info(existing, incoming);

        assert_eq!(merged.rssi, Some(-31));
    }

    #[test]
    fn candidate_merge_uses_stronger_known_rssi() {
        let existing = ScanCandidateInfo {
            name: "yundrone-bw0uwj".to_string(),
            rssi: Some(-72),
        };
        let incoming = ScanCandidateInfo {
            name: "yundrone-bw0uwj".to_string(),
            rssi: Some(-31),
        };

        let merged = super::merge_candidate_info(existing, incoming);

        assert_eq!(merged.rssi, Some(-31));
    }
}
