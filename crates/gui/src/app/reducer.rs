use super::model::{AppModel, UiEvent};

pub fn reduce(model: &mut AppModel, event: UiEvent) {
    match event {
        UiEvent::Log(text) => push_log(model, text),
        UiEvent::ScanStarted => {
            model.is_scanning = true;
            model.is_connected = false;
            model.connected_device_name = None;
            model.scan_candidates.clear();
        }
        UiEvent::ScanResults(candidates) => {
            model.is_scanning = false;
            model.is_connected = false;
            model.scan_candidates = candidates;
        }
        UiEvent::ConnectedDeviceSelected(name) => {
            model.is_scanning = false;
            model.is_connected = true;
            model.connected_device_name = Some(name);
            model.scan_candidates.clear();
        }
        UiEvent::WifiScanLoaded(networks) => {
            model.wifi_list = networks;
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
        UiEvent::Error(err) => {
            model.is_scanning = false;
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
