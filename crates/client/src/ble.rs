use anyhow::{anyhow, Result};
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::collections::HashSet;
use std::hash::Hash;
use std::time::Duration;
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
        self.scan_candidates_with_progress(prefix, timeout_secs, |_| {})
            .await
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
        info!("Starting scan for device with prefix '{}'...", prefix);
        info!(scan_prefix = %prefix, timeout_secs, "ble.scan.started");
        let mut events = self.adapter.events().await?;
        self.adapter.start_scan(ScanFilter::default()).await?;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
        let mut named_peripherals = HashSet::new();

        loop {
            match tokio::time::timeout_at(deadline, events.next()).await {
                Ok(Some(CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id))) => {
                    let peripheral = self.adapter.peripheral(&id).await?;
                    let Some(properties) = peripheral.properties().await? else {
                        continue;
                    };
                    let Some(name) = properties.local_name else {
                        continue;
                    };

                    let rssi = properties.rssi;
                    let matches_prefix = name.starts_with(prefix);
                    debug!(
                        device_name = %name,
                        rssi = ?rssi,
                        matches_prefix,
                        "ble.scan.found"
                    );

                    if mark_peripheral_seen(&mut named_peripherals, id.clone()) {
                        on_progress(progress_event(name.clone(), rssi, matches_prefix));
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => break,
            }
        }

        let stop_result = self.adapter.stop_scan().await;
        stop_result?;
        let peripherals = self.adapter.peripherals().await?;
        let mut filtered_devices = Vec::new();

        for peripheral in peripherals {
            if let Some(properties) = peripheral.properties().await? {
                if let Some(name) = properties.local_name {
                    if name.starts_with(prefix) {
                        filtered_devices.push(ScannedDevice {
                            info: ScanCandidateInfo {
                                name,
                                rssi: properties.rssi,
                            },
                            peripheral,
                        });
                    }
                }
            }
        }

        filtered_devices.sort_by(scan_candidate_cmp);

        if filtered_devices.is_empty() {
            return Err(anyhow!(
                "Device '{}' not found after {}s scan",
                prefix,
                timeout_secs
            ));
        }

        info!(
            scan_prefix = %prefix,
            candidate_count = filtered_devices.len(),
            "ble.scan.completed"
        );

        Ok(filtered_devices)
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
        mark_peripheral_seen, progress_event, sort_scan_candidates, ScanCandidateInfo,
        ScanProgressEvent,
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
}
