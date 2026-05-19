const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

pub fn emit_block(block: &str) {
    println!("{block}");
}

fn color(code: &'static str) -> &'static str {
    match std::env::var("YUNDRONE_LOG_COLOR") {
        Ok(value) if value.eq_ignore_ascii_case("never") => "",
        _ => code,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkMode {
    Single,
    ResponseJson,
}

impl ChunkMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::ResponseJson => "response_json",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Single => "single frame",
            Self::ResponseJson => "chunked response_json",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseLogView<'a> {
    pub request_id: &'a str,
    pub command_name: &'a str,
    pub response_code: &'a str,
    pub response_ok: bool,
    pub response_bytes: Option<usize>,
    pub payload_limit: usize,
    pub chunk_mode: ChunkMode,
    pub chunk_sizes: &'a [usize],
}

#[derive(Debug, Clone)]
pub struct AdvertisingLogView<'a> {
    pub title: &'a str,
    pub adapter_name: &'a str,
    pub identity_name: &'a str,
    pub phase: &'a str,
    pub min_interval: &'a str,
    pub max_interval: &'a str,
    pub tx_power: &'a str,
}

pub fn response_block(view: &ResponseLogView<'_>) -> String {
    let reset = color(RESET);
    let dim = color(DIM);
    let cyan = color(CYAN);
    let max_chunk = view.chunk_sizes.iter().copied().max().unwrap_or(0);
    let status_color = if view.response_ok { GREEN } else { RED };
    let transport_color = if view.chunk_mode == ChunkMode::Single {
        GREEN
    } else {
        YELLOW
    };

    format!(
        "{cyan}BLE response{reset}\n  {dim}request:{reset} {} / {}\n  {dim}result:{reset} {}{} ok={}{}\n  {dim}transport:{reset} {}{}{}\n  {dim}bytes:{reset} response={} limit={} max_chunk={}\n  {dim}chunks:{reset} count={} sizes={:?}",
        view.command_name,
        view.request_id,
        color(status_color),
        view.response_code,
        view.response_ok,
        reset,
        color(transport_color),
        view.chunk_mode.label(),
        reset,
        view.response_bytes
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        view.payload_limit,
        max_chunk,
        view.chunk_sizes.len(),
        view.chunk_sizes,
    )
}

pub fn adapter_identity_block(
    adapter_name: &str,
    snapshot: &crate::adapter_identity::AdapterIdentitySnapshot,
) -> String {
    let reset = color(RESET);
    let dim = color(DIM);
    let cyan = color(CYAN);
    let title = if snapshot.changed {
        "BLE adapter identity"
    } else {
        "BLE adapter identity unchanged"
    };
    format!(
        "{cyan}{title}{reset}\n  {dim}adapter:{reset} {adapter_name}\n  {dim}system name:{reset} {system_name}\n  {dim}previous alias:{reset} {previous_alias}\n  {dim}public alias:{reset} {new_alias}",
        cyan = cyan,
        dim = dim,
        reset = reset,
        system_name = snapshot.system_name,
        previous_alias = snapshot.previous_alias,
        new_alias = snapshot.new_alias
    )
}

pub fn adapter_pairing_block(
    adapter_name: &str,
    snapshot: &crate::adapter_pairing::PairingPolicySnapshot,
) -> String {
    let reset = color(RESET);
    let dim = color(DIM);
    let cyan = color(CYAN);
    let status_color = if snapshot.new_pairable { YELLOW } else { GREEN };
    let title = if snapshot.changed {
        "BLE pairing policy applied"
    } else {
        "BLE pairing policy unchanged"
    };

    format!(
        "{cyan}{title}{reset}\n  {dim}adapter:{reset} {adapter_name}\n  {dim}previous pairable:{reset} {previous_pairable}\n  {dim}current pairable:{reset} {status_color}{new_pairable}{reset}\n  {dim}mode:{reset} connectable GATT, no system pairing",
        previous_pairable = snapshot.previous_pairable,
        new_pairable = snapshot.new_pairable,
        status_color = color(status_color),
    )
}

pub fn startup_block(
    adapter_name: &str,
    identity_name: &str,
    identity_source: &str,
    advertising_backend: &str,
) -> String {
    let reset = color(RESET);
    let dim = color(DIM);
    let cyan = color(CYAN);
    format!(
        "{cyan}BLE server{reset}\n  {dim}adapter:{reset} {adapter_name}\n  {dim}identity:{reset} {identity_name}\n  {dim}identity source:{reset} {identity_source}\n  {dim}advertising backend:{reset} {advertising_backend}"
    )
}

pub fn advertising_block(view: &AdvertisingLogView<'_>) -> String {
    let reset = color(RESET);
    let dim = color(DIM);
    let cyan = color(CYAN);
    format!(
        "{cyan}{title}{reset}\n  {dim}adapter:{reset} {adapter_name}\n  {dim}identity:{reset} {identity_name}\n  {dim}phase:{reset} {phase}\n  {dim}interval:{reset} {min_interval}..{max_interval}\n  {dim}tx_power:{reset} {tx_power}",
        title = view.title,
        adapter_name = view.adapter_name,
        identity_name = view.identity_name,
        phase = view.phase,
        min_interval = view.min_interval,
        max_interval = view.max_interval,
        tx_power = view.tx_power,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_block_includes_chunk_details_without_payload() {
        let sizes = vec![352, 356, 219];
        let block = response_block(&ResponseLogView {
            request_id: "req-1",
            command_name: "wifi.scan",
            response_code: "OK",
            response_ok: true,
            response_bytes: Some(927),
            payload_limit: 360,
            chunk_mode: ChunkMode::ResponseJson,
            chunk_sizes: &sizes,
        });

        assert!(block.contains("BLE response"));
        assert!(block.contains("wifi.scan / req-1"));
        assert!(block.contains("chunked response_json"));
        assert!(block.contains("response=927 limit=360 max_chunk=356"));
        assert!(block.contains("count=3 sizes=[352, 356, 219]"));
        assert!(!block.contains("payload"));
    }
}
