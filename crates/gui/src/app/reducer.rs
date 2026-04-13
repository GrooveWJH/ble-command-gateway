use super::model::{ActionPhase, AppModel, UiEvent};

pub fn reduce(model: &mut AppModel, event: UiEvent) {
    match event {
        UiEvent::Log(text) => push_log(model, text),
        UiEvent::ActionStarted { slot, request_id } => {
            model.active_action = Some(slot);
            model.record_action_feedback(slot, ActionPhase::Running, request_id, None, None);
        }
        UiEvent::ActionSucceeded {
            slot,
            request_id,
            detail,
        } => {
            if model.active_action == Some(slot) {
                model.active_action = None;
            }
            model.record_action_feedback(slot, ActionPhase::Succeeded, request_id, detail, None);
        }
        UiEvent::ActionFailed {
            slot,
            request_id,
            error,
        } => {
            if model.active_action == Some(slot) {
                model.active_action = None;
            }
            model.record_action_feedback(slot, ActionPhase::Failed, request_id, None, Some(error));
        }
        UiEvent::ScanStarted => {
            model.is_scanning = true;
            model.is_connecting = false;
            model.is_connected = false;
            model.heartbeat_ok = false;
            model.heartbeat_failures = 0;
            model.last_heartbeat_at = None;
            model.connected_device_name = None;
            model.scan_candidates.clear();
        }
        UiEvent::ScanCandidateDiscovered(candidate) => {
            model.scan_candidates.push(candidate);
        }
        UiEvent::ScanFinished | UiEvent::ScanStopped => {
            model.is_scanning = false;
        }
        UiEvent::ConnectingToCandidate(name) => {
            model.is_scanning = false;
            model.is_connecting = true;
            model.is_connected = false;
            model.heartbeat_ok = false;
            model.heartbeat_failures = 0;
            model.last_heartbeat_at = None;
            model.connected_device_name = Some(name);
        }
        UiEvent::ConnectedDeviceSelected(name) => {
            model.is_scanning = false;
            model.is_connecting = false;
            model.is_connected = true;
            model.heartbeat_ok = true;
            model.heartbeat_failures = 0;
            model.last_heartbeat_at = None;
            model.connected_device_name = Some(name);
            model.scan_candidates.clear();
        }
        UiEvent::HeartbeatOk { at } => {
            model.heartbeat_ok = true;
            model.heartbeat_failures = 0;
            model.last_heartbeat_at = Some(at);
        }
        UiEvent::HeartbeatMissed(failures) => {
            model.heartbeat_ok = false;
            model.heartbeat_failures = failures;
        }
        UiEvent::Disconnected { .. } => {
            model.is_scanning = false;
            model.is_connecting = false;
            model.is_connected = false;
            model.active_action = None;
            model.heartbeat_ok = false;
            model.heartbeat_failures = 0;
            model.last_heartbeat_at = None;
            model.connected_device_name = None;
            model.scan_candidates.clear();
        }
        UiEvent::WifiScanLoaded(networks) => {
            model.wifi_list = networks;
        }
        UiEvent::DiagnosticResult(result) => {
            model.diagnostic_result = Some(result);
        }
        UiEvent::ProvisionResult(result) => {
            model.provision_result = Some(result);
        }
        UiEvent::CommandCompleted(result) => {
            push_log(
                model,
                format!(
                    "[CMD] {} {}: {}",
                    result.request_id, result.code, result.text
                ),
            );
        }
        UiEvent::ConnectionFailed(err) => {
            model.is_scanning = false;
            model.is_connecting = false;
            model.is_connected = false;
            model.active_action = None;
            model.heartbeat_ok = false;
            model.heartbeat_failures = 0;
            model.last_heartbeat_at = None;
            model.connected_device_name = None;
            model.scan_candidates.clear();
            push_log(model, format!("[ERR] {}", err));
        }
        UiEvent::Error(err) => {
            model.is_scanning = false;
            model.is_connecting = false;
            push_log(model, format!("[ERR] {}", err));
        }
    }
}

fn push_log(model: &mut AppModel, text: String) {
    model.logs.push(text);
    if model.logs.len() > 100 {
        model.logs.remove(0);
    }
}
