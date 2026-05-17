use anyhow::{anyhow, Result};
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, info};

use crate::discovery::{classify_properties, DiscoveryCriteria};
use crate::scan_state::{build_scanned_device, LiveScanState};

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
        let criteria = DiscoveryCriteria::for_prefix(prefix);
        info!("Starting scan for device with prefix '{}'...", prefix);
        info!(scan_prefix = %prefix, timeout_secs, "ble.scan.started");
        let mut events = self.adapter.events().await?;
        self.adapter
            .start_scan(ScanFilter {
                services: vec![criteria.service_uuid],
            })
            .await?;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
        let mut live_state = LiveScanState::new();
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
                            let Some(matched) = classify_properties(&properties, &criteria) else {
                                continue;
                            };

                            let rssi = properties.rssi;
                            let observation = live_state.observe(
                                id.clone(),
                                &matched.display_name,
                                matched.candidate_name,
                                matched.matches_identity,
                                rssi,
                            );

                            debug!(
                                device_name = %matched.display_name,
                                rssi = ?rssi,
                                matches_prefix = observation.matches_identity,
                                "ble.scan.found"
                            );

                            if let Some(progress) = observation.progress {
                                on_progress(progress);
                            }

                            if let Some(candidate_name) = observation.candidate_name {
                                on_candidate(build_scanned_device(peripheral, candidate_name, rssi));
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
            candidate_count = live_state.candidate_count,
            cancelled,
            "ble.scan.completed"
        );

        Ok(ScanRunSummary {
            named_device_count: live_state.named_device_count,
            candidate_count: live_state.candidate_count,
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

pub(crate) fn progress_event(
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
