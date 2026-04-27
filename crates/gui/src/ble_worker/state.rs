use client::{BleClient, ScannedDevice};
use std::time::{Duration, Instant};
use tokio::sync::watch;

const HEARTBEAT_DISCONNECT_GRACE_SECS: u64 = 12;

#[derive(Default)]
pub(crate) struct WorkerState {
    discovered_client: Option<BleClient>,
    discovered_devices: Vec<ScannedDevice>,
    active_session: Option<client::BleSession>,
    active_scan_id: Option<u64>,
    next_scan_id: u64,
    scan_cancel_tx: Option<watch::Sender<bool>>,
    heartbeat_failures: u8,
    heartbeat_abnormal_since: Option<Instant>,
    heartbeat_disconnect_deadline: Option<Instant>,
}

impl WorkerState {
    pub(super) fn begin_scan(&mut self, client: BleClient, cancel_tx: watch::Sender<bool>) -> u64 {
        let scan_id = self.next_scan_id;
        self.next_scan_id += 1;
        self.discovered_client = Some(client);
        self.discovered_devices.clear();
        self.active_session = None;
        self.active_scan_id = Some(scan_id);
        self.scan_cancel_tx = Some(cancel_tx);
        self.reset_heartbeat_failures();
        scan_id
    }

    pub(super) fn add_discovered_device(&mut self, device: ScannedDevice) {
        if self
            .discovered_devices
            .iter()
            .any(|candidate| candidate.info.name == device.info.name)
        {
            return;
        }
        self.discovered_devices.push(device);
    }

    pub(super) fn finish_scan(&mut self, scan_id: u64) -> bool {
        if self.active_scan_id == Some(scan_id) {
            self.active_scan_id = None;
            self.scan_cancel_tx = None;
            return true;
        }
        false
    }

    pub(super) fn cancel_scan(&mut self) -> bool {
        let Some(cancel_tx) = self.scan_cancel_tx.take() else {
            return false;
        };
        let _ = cancel_tx.send(true);
        self.active_scan_id = None;
        true
    }

    pub(super) fn is_active_scan(&self, scan_id: u64) -> bool {
        self.active_scan_id == Some(scan_id)
    }

    pub(super) fn take_connection_target(
        &mut self,
        name: &str,
    ) -> Result<(BleClient, ScannedDevice), String> {
        let client = self
            .discovered_client
            .take()
            .ok_or_else(|| "No scanned devices available".to_string())?;
        let device = self
            .discovered_devices
            .iter()
            .find(|candidate| candidate.info.name == name)
            .cloned()
            .ok_or_else(|| format!("Scanned device '{}' is no longer available", name))?;
        Ok((client, device))
    }

    pub(super) fn activate_session(&mut self, session: client::BleSession) {
        self.active_session = Some(session);
        self.discovered_client = None;
        self.discovered_devices.clear();
        self.active_scan_id = None;
        self.scan_cancel_tx = None;
        self.reset_heartbeat_failures();
    }

    pub(super) fn active_session_mut(&mut self) -> Option<&mut client::BleSession> {
        self.active_session.as_mut()
    }

    pub(super) fn active_session(&self) -> Option<&client::BleSession> {
        self.active_session.as_ref()
    }

    pub(super) fn take_active_session(&mut self) -> Option<client::BleSession> {
        self.active_session.take()
    }

    pub(super) fn has_active_session(&self) -> bool {
        self.active_session.is_some()
    }

    pub(super) fn reset_heartbeat_failures(&mut self) {
        self.heartbeat_failures = 0;
        self.heartbeat_abnormal_since = None;
        self.heartbeat_disconnect_deadline = None;
    }

    pub(crate) fn record_heartbeat_failure(&mut self, now: Instant) -> u8 {
        if self.heartbeat_abnormal_since.is_none() {
            self.heartbeat_abnormal_since = Some(now);
            self.heartbeat_disconnect_deadline =
                Some(now + Duration::from_secs(HEARTBEAT_DISCONNECT_GRACE_SECS));
        }
        self.heartbeat_failures = self.heartbeat_failures.saturating_add(1);
        self.heartbeat_failures
    }

    #[cfg(test)]
    pub(crate) fn heartbeat_disconnect_deadline(&self) -> Option<Instant> {
        self.heartbeat_disconnect_deadline
    }

    pub(super) fn heartbeat_failures(&self) -> u8 {
        self.heartbeat_failures
    }

    pub(super) fn heartbeat_deadline_elapsed(&self, now: Instant) -> bool {
        self.heartbeat_disconnect_deadline
            .map(|deadline| now >= deadline)
            .unwrap_or(false)
    }

    pub(super) fn reset_to_idle(&mut self) {
        self.active_session = None;
        self.discovered_client = None;
        self.discovered_devices.clear();
        self.active_scan_id = None;
        self.scan_cancel_tx = None;
        self.reset_heartbeat_failures();
    }
}

pub(super) enum ScanWorkerEvent {
    Progress {
        scan_id: u64,
        event: client::ScanProgressEvent,
    },
    Candidate {
        scan_id: u64,
        device: ScannedDevice,
    },
    Finished {
        scan_id: u64,
        summary: client::ScanRunSummary,
    },
    Failed {
        scan_id: u64,
        error: String,
    },
}
