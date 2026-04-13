use anyhow::{anyhow, Result};
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::collections::HashSet;
use std::hash::Hash;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, info};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanCandidateInfo {
    pub name: String,
    pub rssi: Option<i16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanProgressEvent {
    pub device_name: String,
    pub rssi: Option<i16>,
    pub matches_prefix: bool,
}

#[derive(Clone, Debug)]
pub struct ScannedDevice {
    pub info: ScanCandidateInfo,
    pub peripheral: Peripheral,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanRunSummary {
    pub named_device_count: usize,
    pub candidate_count: usize,
    pub cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScanObservation {
    progress: Option<ScanProgressEvent>,
    candidate_name: Option<String>,
}

#[derive(Clone)]
pub struct BleClient {
    adapter: Adapter,
}

impl BleClient {
    pub async fn new() -> Result<Self> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;

        let adapter = adapters
            .into_iter()
            .nth(0)
            .ok_or_else(|| anyhow!("No Bluetooth adapters found"))?;

        Ok(Self { adapter })
    }

    pub async fn scan_candidates(
        &self,
        prefix: &str,
        timeout_secs: u64,
    ) -> Result<Vec<ScannedDevice>> {
        let (_cancel_tx, mut cancel_rx) = watch::channel(false);
        let mut candidates = Vec::new();
        let summary = self
            .scan_candidates_live(
                prefix,
                timeout_secs,
                &mut cancel_rx,
                |_| {},
                |device| {
                    candidates.push(device);
                },
            )
            .await?;

        if summary.candidate_count == 0 {
            return Err(anyhow!(
                "Device '{}' not found after {}s scan",
                prefix,
                timeout_secs
            ));
        }

        candidates.sort_by(scan_candidate_cmp);
        Ok(candidates)
    }

    pub async fn scan_candidates_with_progress<F>(
        &self,
        prefix: &str,
        timeout_secs: u64,
        mut on_progress: F,
    ) -> Result<Vec<ScannedDevice>>
    where
        F: FnMut(ScanProgressEvent),
    {
        let (_cancel_tx, mut cancel_rx) = watch::channel(false);
        let mut filtered_devices = Vec::new();
        let summary = self
            .scan_candidates_live(
                prefix,
                timeout_secs,
                &mut cancel_rx,
                &mut on_progress,
                |device| filtered_devices.push(device),
            )
            .await?;

        if summary.candidate_count == 0 {
            return Err(anyhow!(
                "Device '{}' not found after {}s scan",
                prefix,
                timeout_secs
            ));
        }

        filtered_devices.sort_by(scan_candidate_cmp);
        Ok(filtered_devices)
    }

    pub async fn scan_candidates_live<FP, FC>(
        &self,
        prefix: &str,
        timeout_secs: u64,
        cancel_rx: &mut watch::Receiver<bool>,
        mut on_progress: FP,
        mut on_candidate: FC,
    ) -> Result<ScanRunSummary>
    where
        FP: FnMut(ScanProgressEvent),
        FC: FnMut(ScannedDevice),
    {
        info!("Starting scan for device with prefix '{}'...", prefix);
        info!(scan_prefix = %prefix, timeout_secs, "ble.scan.started");
        let mut events = self.adapter.events().await?;
        self.adapter.start_scan(ScanFilter::default()).await?;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
        let mut named_peripherals = HashSet::new();
        let mut matched_peripherals = HashSet::new();
        let mut named_device_count = 0usize;
        let mut candidate_count = 0usize;
        let mut cancelled = false;

        loop {
            tokio::select! {
                changed = cancel_rx.changed() => {
                    match changed {
                        Ok(()) if *cancel_rx.borrow() => {
                            cancelled = true;
                            break;
                        }
                        Ok(()) => {}
                        Err(_) => break,
                    }
                }
                event = tokio::time::timeout_at(deadline, events.next()) => {
                    match event {
                        Ok(Some(CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id))) => {
                            let peripheral = self.adapter.peripheral(&id).await?;
                            let Some(properties) = peripheral.properties().await? else {
                                continue;
                            };
                            let Some(name) = properties.local_name else {
                                continue;
                            };

                            let rssi = properties.rssi;
                            let observation = classify_scan_observation(
                                &mut named_peripherals,
                                &mut matched_peripherals,
                                id.clone(),
                                &name,
                                prefix,
                                rssi,
                            );

                            debug!(
                                device_name = %name,
                                rssi = ?rssi,
                                matches_prefix = observation.candidate_name.is_some(),
                                "ble.scan.found"
                            );

                            if let Some(progress) = observation.progress {
                                named_device_count += 1;
                                on_progress(progress);
                            }

                            if let Some(candidate_name) = observation.candidate_name {
                                candidate_count += 1;
                                on_candidate(ScannedDevice {
                                    info: ScanCandidateInfo {
                                        name: candidate_name,
                                        rssi,
                                    },
                                    peripheral,
                                });
                            }
                        }
                        Ok(Some(_)) => {}
                        Ok(None) | Err(_) => break,
                    }
                }
            }
        }

        self.adapter.stop_scan().await?;
        info!(
            scan_prefix = %prefix,
            candidate_count,
            cancelled,
            "ble.scan.completed"
        );

        Ok(ScanRunSummary {
            named_device_count,
            candidate_count,
            cancelled,
        })
    }

    pub async fn connect_session(
        &self,
        device: ScannedDevice,
    ) -> Result<crate::session::BleSession> {
        info!(
            device_name = %device.info.name,
            rssi = ?device.info.rssi,
            "ble.connect.started"
        );
        crate::session::BleSession::connect(device.info.name, device.info.rssi, device.peripheral)
            .await
    }
}

fn progress_event(
    device_name: String,
    rssi: Option<i16>,
    matches_prefix: bool,
) -> ScanProgressEvent {
    ScanProgressEvent {
        device_name,
        rssi,
        matches_prefix,
    }
}

fn mark_peripheral_seen<K>(seen: &mut HashSet<K>, peripheral_id: K) -> bool
where
    K: Eq + Hash,
{
    seen.insert(peripheral_id)
}

fn classify_scan_observation<K>(
    named_seen: &mut HashSet<K>,
    matched_seen: &mut HashSet<K>,
    peripheral_id: K,
    raw_name: &str,
    prefix: &str,
    rssi: Option<i16>,
) -> ScanObservation
where
    K: Eq + Hash + Clone,
{
    let candidate_name = extract_prefixed_name(raw_name, prefix);
    let matches_prefix = candidate_name.is_some();
    let progress = if mark_peripheral_seen(named_seen, peripheral_id.clone()) {
        Some(progress_event(raw_name.to_string(), rssi, matches_prefix))
    } else {
        None
    };
    let candidate_name =
        candidate_name.filter(|_| mark_peripheral_seen(matched_seen, peripheral_id));

    ScanObservation {
        progress,
        candidate_name,
    }
}

fn extract_prefixed_name(raw_name: &str, prefix: &str) -> Option<String> {
    if raw_name.starts_with(prefix) {
        return Some(raw_name.to_string());
    }

    // Some stacks expose local_name like `host-name [Yundrone_UAV-15-19-A7F2]`.
    // In that case we treat the bracketed BLE name as the matching candidate.
    let start = raw_name.find('[')?;
    let end = raw_name.rfind(']')?;
    if end <= start + 1 {
        return None;
    }

    let inner = raw_name[start + 1..end].trim();
    if inner.starts_with(prefix) {
        Some(inner.to_string())
    } else {
        None
    }
}

pub fn sort_scan_candidates(candidates: &mut [ScanCandidateInfo]) {
    candidates.sort_by(scan_candidate_info_cmp);
}

fn scan_candidate_cmp(left: &ScannedDevice, right: &ScannedDevice) -> std::cmp::Ordering {
    scan_candidate_info_cmp(&left.info, &right.info)
}

fn scan_candidate_info_cmp(
    left: &ScanCandidateInfo,
    right: &ScanCandidateInfo,
) -> std::cmp::Ordering {
    right
        .rssi
        .unwrap_or(i16::MIN)
        .cmp(&left.rssi.unwrap_or(i16::MIN))
        .then_with(|| left.name.cmp(&right.name))
}

#[cfg(test)]
mod tests {
    use super::{
        classify_scan_observation, extract_prefixed_name, mark_peripheral_seen, progress_event,
        sort_scan_candidates, ScanCandidateInfo, ScanProgressEvent,
    };
    use std::collections::HashSet;

    #[test]
    fn sort_scan_candidates_orders_devices_by_signal_then_name() {
        let mut candidates = vec![
            ScanCandidateInfo {
                name: "Yundrone_UAV-15-19-A7F2".to_string(),
                rssi: Some(-60),
            },
            ScanCandidateInfo {
                name: "Other_Device".to_string(),
                rssi: Some(-20),
            },
            ScanCandidateInfo {
                name: "Yundrone_UAV-15-20-B110".to_string(),
                rssi: Some(-45),
            },
        ];

        sort_scan_candidates(&mut candidates);

        assert_eq!(candidates[0].name, "Other_Device");
        assert_eq!(candidates[1].name, "Yundrone_UAV-15-20-B110");
        assert_eq!(candidates[2].name, "Yundrone_UAV-15-19-A7F2");
    }

    #[test]
    fn sort_scan_candidates_puts_unknown_rssi_last() {
        let mut candidates = vec![
            ScanCandidateInfo {
                name: "Yundrone_UAV-15-21-UNK".to_string(),
                rssi: None,
            },
            ScanCandidateInfo {
                name: "Yundrone_UAV-15-20-B110".to_string(),
                rssi: Some(-45),
            },
        ];

        sort_scan_candidates(&mut candidates);

        assert_eq!(candidates[0].name, "Yundrone_UAV-15-20-B110");
        assert_eq!(candidates[1].name, "Yundrone_UAV-15-21-UNK");
    }

    #[test]
    fn progress_event_captures_name_signal_and_prefix_match() {
        assert_eq!(
            progress_event("Yundrone_UAV-15-19-A7".to_string(), Some(-41), true),
            ScanProgressEvent {
                device_name: "Yundrone_UAV-15-19-A7".to_string(),
                rssi: Some(-41),
                matches_prefix: true,
            }
        );
    }

    #[test]
    fn mark_peripheral_seen_only_accepts_first_sighting() {
        let mut seen = HashSet::new();

        assert!(mark_peripheral_seen(&mut seen, "dev-1".to_string()));
        assert!(!mark_peripheral_seen(&mut seen, "dev-1".to_string()));
        assert!(mark_peripheral_seen(&mut seen, "dev-2".to_string()));
    }

    #[test]
    fn extract_prefixed_name_accepts_direct_prefix_name() {
        let result = extract_prefixed_name("Yundrone_UAV-15-19-A7", "Yundrone_UAV");
        assert_eq!(result.as_deref(), Some("Yundrone_UAV-15-19-A7"));
    }

    #[test]
    fn extract_prefixed_name_accepts_bracketed_prefix_name() {
        let result =
            extract_prefixed_name("orangepi4pro [Yundrone_UAV-03-17-5433]", "Yundrone_UAV");
        assert_eq!(result.as_deref(), Some("Yundrone_UAV-03-17-5433"));
    }

    #[test]
    fn extract_prefixed_name_rejects_non_matching_name() {
        let result = extract_prefixed_name("GrooveiPhone", "Yundrone_UAV");
        assert!(result.is_none());
    }

    #[test]
    fn classify_scan_observation_emits_candidate_only_once_per_device() {
        let mut named_seen = HashSet::new();
        let mut matched_seen = HashSet::new();

        let first = classify_scan_observation(
            &mut named_seen,
            &mut matched_seen,
            "dev-1".to_string(),
            "orangepi4pro [Yundrone_UAV-03-17-5433]",
            "Yundrone_UAV",
            Some(-11),
        );
        let second = classify_scan_observation(
            &mut named_seen,
            &mut matched_seen,
            "dev-1".to_string(),
            "orangepi4pro [Yundrone_UAV-03-17-5433]",
            "Yundrone_UAV",
            Some(-9),
        );

        assert!(first.progress.is_some());
        assert_eq!(
            first.candidate_name.as_deref(),
            Some("Yundrone_UAV-03-17-5433")
        );
        assert!(second.progress.is_none());
        assert!(second.candidate_name.is_none());
    }

    #[test]
    fn classify_scan_observation_keeps_non_matching_devices_out_of_candidate_list() {
        let mut named_seen = HashSet::new();
        let mut matched_seen = HashSet::new();

        let observation = classify_scan_observation(
            &mut named_seen,
            &mut matched_seen,
            "dev-2".to_string(),
            "GrooveiPhone",
            "Yundrone_UAV",
            Some(-51),
        );

        assert!(observation.progress.is_some());
        assert!(observation.candidate_name.is_none());
    }
}
