use anyhow::{anyhow, Context as _, Result};
use btleplug::api::{
    Central, CentralEvent, CharPropFlags, Manager as _, Peripheral as _, PeripheralProperties,
    ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

use client::discovery::{classify_properties, DiscoveryCriteria, UART_SERVICE_UUID};

const WRITE_UUID: &str = "6E400002-B5A3-F393-E0A9-E50E24DCCA9E";
const NOTIFY_UUID: &str = "6E400003-B5A3-F393-E0A9-E50E24DCCA9E";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugStep {
    pub name: &'static str,
    pub detail: String,
}

impl DebugStep {
    pub fn ok(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            detail: detail.into(),
        }
    }

    pub fn line(&self) -> String {
        format!("[OK] {:<18} {}", self.name, self.detail)
    }
}

async fn with_step_timeout<T>(
    step: &str,
    timeout_secs: u64,
    future: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::time::timeout(Duration::from_secs(timeout_secs), future)
        .await
        .map_err(|_| anyhow!("{step} timed out after {timeout_secs}s"))?
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugPeripheralSummary {
    pub local_name: String,
    pub rssi: Option<i16>,
    pub address: String,
    pub advertised_services: Vec<String>,
}

pub fn format_peripheral_summary(summary: &DebugPeripheralSummary) -> String {
    let services = if summary.advertised_services.is_empty() {
        "none".to_string()
    } else {
        summary.advertised_services.join(", ")
    };
    format!(
        "name={} rssi={} address={} advertised_services=[{}]",
        summary.local_name,
        summary
            .rssi
            .map(|value| format!("{value} dBm"))
            .unwrap_or_else(|| "unknown".to_string()),
        summary.address,
        services
    )
}

struct DebugLog {
    lines: Vec<String>,
    output: Option<PathBuf>,
}

impl DebugLog {
    fn new(output: Option<PathBuf>) -> Self {
        Self {
            lines: Vec::new(),
            output,
        }
    }

    fn line(&mut self, line: impl Into<String>) {
        let line = line.into();
        println!("{line}");
        self.lines.push(line);
        let _ = self.flush();
    }

    fn flush(&self) -> Result<()> {
        if let Some(path) = &self.output {
            std::fs::write(path, format!("{}\n", self.lines.join("\n")))
                .with_context(|| format!("write debug log {}", path.display()))?;
        }
        Ok(())
    }
}

pub async fn run(
    prefix: &str,
    timeout_secs: u64,
    response_timeout_secs: u64,
    output: Option<PathBuf>,
    trace_chunks: bool,
) -> Result<()> {
    let mut log = DebugLog::new(output);
    match run_with_log(
        prefix,
        timeout_secs,
        response_timeout_secs,
        trace_chunks,
        &mut log,
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(err) => {
            log.line(format!("[ERR] {err:#}"));
            Err(err)
        }
    }
}

async fn run_with_log(
    prefix: &str,
    timeout_secs: u64,
    response_timeout_secs: u64,
    trace_chunks: bool,
    log: &mut DebugLog,
) -> Result<()> {
    let criteria = DiscoveryCriteria::for_prefix(prefix);
    let service_uuid = Uuid::parse_str(UART_SERVICE_UUID)?;
    let write_uuid = Uuid::parse_str(WRITE_UUID)?;
    let notify_uuid = Uuid::parse_str(NOTIFY_UUID)?;

    log.line(">>> YunDrone BLE Debug CLI <<<");
    log.line(format!("target prefix: {prefix}"));
    log.line(format!("scan timeout: {timeout_secs}s"));
    log.line(format!("response timeout: {response_timeout_secs}s"));

    let manager = Manager::new().await.context("create BLE manager")?;
    let adapters = manager.adapters().await.context("list BLE adapters")?;
    let adapter = adapters
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No Bluetooth adapters found"))?;
    log.line(DebugStep::ok("adapter", format!("{adapter:?}")).line());

    let (peripheral, properties) =
        find_target_peripheral(&adapter, &criteria, timeout_secs, log).await?;
    let summary = summarize_properties(&properties);
    log.line(DebugStep::ok("scan-match", format_peripheral_summary(&summary)).line());

    let started = Instant::now();
    with_step_timeout("connect peripheral", response_timeout_secs, async {
        peripheral.connect().await.context("connect peripheral")
    })
    .await?;
    log.line(DebugStep::ok("connect", format!("{} ms", started.elapsed().as_millis())).line());

    let started = Instant::now();
    with_step_timeout("discover GATT services", response_timeout_secs, async {
        peripheral
            .discover_services()
            .await
            .context("discover GATT services")
    })
    .await?;
    log.line(
        DebugStep::ok(
            "discover",
            format!(
                "{} ms, {} services",
                started.elapsed().as_millis(),
                peripheral.services().len()
            ),
        )
        .line(),
    );

    print_services(&peripheral, service_uuid, log);

    let chars = peripheral.characteristics();
    let write_char = chars
        .iter()
        .find(|char| {
            char.uuid == write_uuid
                && char
                    .properties
                    .intersects(CharPropFlags::WRITE | CharPropFlags::WRITE_WITHOUT_RESPONSE)
        })
        .cloned()
        .ok_or_else(|| anyhow!("UART write characteristic not found after discovery"))?;
    let notify_char = chars
        .iter()
        .find(|char| {
            char.uuid == notify_uuid
                && char
                    .properties
                    .intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE)
        })
        .cloned()
        .ok_or_else(|| anyhow!("UART notify characteristic not found after discovery"))?;
    log.line(
        DebugStep::ok(
            "uart-chars",
            format!(
                "write={} props={:?}; notify={} props={:?}",
                write_char.uuid, write_char.properties, notify_char.uuid, notify_char.properties
            ),
        )
        .line(),
    );

    let started = Instant::now();
    with_step_timeout(
        "subscribe UART notify characteristic",
        response_timeout_secs,
        async {
            peripheral
                .subscribe(&notify_char)
                .await
                .context("subscribe UART notify characteristic")
        },
    )
    .await?;
    log.line(DebugStep::ok("subscribe", format!("{} ms", started.elapsed().as_millis())).line());
    let mut notifications = peripheral
        .notifications()
        .await
        .context("open notification stream")?;

    run_probe_command(
        &peripheral,
        &write_char,
        &mut notifications,
        protocol::requests::CommandPayload::LinkHeartbeat,
        response_timeout_secs,
        trace_chunks,
        log,
    )
    .await?;
    run_probe_command(
        &peripheral,
        &write_char,
        &mut notifications,
        protocol::requests::CommandPayload::SystemCapabilities,
        response_timeout_secs,
        trace_chunks,
        log,
    )
    .await?;

    peripheral
        .unsubscribe(&notify_char)
        .await
        .context("unsubscribe UART notify characteristic")?;
    peripheral
        .disconnect()
        .await
        .context("disconnect peripheral")?;
    log.line(DebugStep::ok("disconnect", "clean").line());
    log.flush()?;
    Ok(())
}

async fn find_target_peripheral(
    adapter: &Adapter,
    criteria: &DiscoveryCriteria,
    timeout_secs: u64,
    log: &mut DebugLog,
) -> Result<(Peripheral, PeripheralProperties)> {
    let mut events = adapter.events().await.context("open BLE event stream")?;
    adapter
        .start_scan(ScanFilter { services: vec![] })
        .await
        .context("start BLE scan")?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut result = None;
    loop {
        match tokio::time::timeout_at(deadline, events.next()).await {
            Ok(Some(CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id))) => {
                let peripheral = adapter.peripheral(&id).await?;
                let Some(properties) = peripheral.properties().await? else {
                    continue;
                };
                let Some(discovery) = classify_properties(&properties, criteria) else {
                    continue;
                };
                log.line(format!(
                    "[SCAN] {} rssi={:?} candidate={}",
                    discovery.display_name, properties.rssi, discovery.matches_identity
                ));
                if discovery.matches_identity {
                    result = Some((peripheral, properties));
                    break;
                }
            }
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }

    adapter.stop_scan().await.context("stop BLE scan")?;
    result.ok_or_else(|| {
        anyhow!(
            "No BLE device matching prefix '{}' found after {}s",
            criteria.stable_prefix,
            timeout_secs
        )
    })
}

fn summarize_properties(properties: &PeripheralProperties) -> DebugPeripheralSummary {
    let mut services = properties
        .services
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    services.sort();

    DebugPeripheralSummary {
        local_name: properties
            .local_name
            .clone()
            .unwrap_or_else(|| "<unnamed>".to_string()),
        rssi: properties.rssi,
        address: properties.address.to_string(),
        advertised_services: services,
    }
}

fn print_services(peripheral: &Peripheral, expected_service_uuid: Uuid, log: &mut DebugLog) {
    let services = peripheral.services();
    for service in &services {
        let marker = if service.uuid == expected_service_uuid {
            " <- UART service"
        } else {
            ""
        };
        log.line(format!(
            "  service {} primary={}{}",
            service.uuid, service.primary, marker
        ));
        print_characteristics(&service.characteristics, log);
    }
}

fn print_characteristics(chars: &BTreeSet<btleplug::api::Characteristic>, log: &mut DebugLog) {
    for char in chars {
        log.line(format!(
            "    char {} props={:?}",
            char.uuid, char.properties
        ));
    }
}

async fn run_probe_command(
    peripheral: &Peripheral,
    write_char: &btleplug::api::Characteristic,
    notifications: &mut std::pin::Pin<
        Box<dyn futures::Stream<Item = btleplug::api::ValueNotification> + Send>,
    >,
    payload: protocol::requests::CommandPayload,
    timeout_secs: u64,
    trace_chunks: bool,
    log: &mut DebugLog,
) -> Result<()> {
    let command_name = payload.command_name();
    let prepared = client::prepare_request(payload)?;
    log.line(
        DebugStep::ok(
            "tx",
            format!(
                "{} id={} bytes={}",
                command_name,
                prepared.request.id,
                prepared.bytes.len()
            ),
        )
        .line(),
    );
    peripheral
        .write(write_char, &prepared.bytes, WriteType::WithoutResponse)
        .await
        .with_context(|| format!("write {command_name} request"))?;

    let mut decoder = client::response::ResponseDecoder::new();
    let response = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
        while let Some(notification) = notifications.next().await {
            if trace_chunks {
                log_notification(&notification.value, log);
            }
            match decoder.decode(&notification.value)? {
                Some(response) if response.id == prepared.request.id => {
                    if trace_chunks {
                        log_reassembled_response(&response, log);
                    }
                    return Ok(response);
                }
                Some(response) => {
                    log.line(format!(
                        "[RX:other] id={} cmd={:?} code={} final={}",
                        response.id, response.cmd, response.code, response.final_flag
                    ));
                }
                None => {}
            }
        }

        Err(anyhow!("notification stream closed"))
    })
    .await
    .map_err(|_| {
        anyhow!("Timed out waiting for {command_name} response after {timeout_secs}s")
    })??;

    log.line(
        DebugStep::ok(
            "rx",
            format!(
                "{} id={} ok={} code={} phase={:?} final={} text={}",
                command_name,
                response.id,
                response.ok,
                response.code,
                response.phase,
                response.final_flag,
                response.text
            ),
        )
        .line(),
    );
    Ok(())
}

fn log_notification(raw: &[u8], log: &mut DebugLog) {
    let text = String::from_utf8_lossy(raw);
    log.line(format!("[RX:raw] bytes={} {}", raw.len(), text));

    match protocol::parse_response(raw) {
        Ok(response) => {
            if let Some(chunk) = response
                .data
                .as_ref()
                .and_then(|data| data.get("chunk"))
                .and_then(|value| value.as_object())
            {
                let mode = chunk
                    .get("mode")
                    .and_then(|value| value.as_str())
                    .unwrap_or("<missing>");
                let index = chunk
                    .get("index")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
                let total = chunk
                    .get("total")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
                let payload = chunk
                    .get("payload")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                log.line(format!(
                    "[RX:chunk] id={} mode={} index={}/{} payload_bytes={} payload={}",
                    response.id,
                    mode,
                    index,
                    total,
                    payload.len(),
                    payload
                ));
            } else {
                log.line(format!(
                    "[RX:frame] id={} cmd={:?} code={} phase={:?} final={} text={}",
                    response.id,
                    response.cmd,
                    response.code,
                    response.phase,
                    response.final_flag,
                    response.text
                ));
            }
        }
        Err(err) => log.line(format!("[RX:parse-error] {err}")),
    }
}

fn log_reassembled_response(response: &protocol::CommandResponse, log: &mut DebugLog) {
    match protocol::encode_response(response) {
        Ok(bytes) => {
            log.line(format!(
                "[RX:assembled] bytes={} {}",
                bytes.len(),
                String::from_utf8_lossy(&bytes)
            ));
        }
        Err(err) => log.line(format!("[RX:assembled-error] {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{format_peripheral_summary, DebugPeripheralSummary, DebugStep};

    #[test]
    fn debug_step_line_is_stable_and_readable() {
        let line = DebugStep::ok("discover", "12 ms, 1 services").line();

        assert!(line.contains("[OK]"));
        assert!(line.contains("discover"));
        assert!(line.contains("12 ms, 1 services"));
    }

    #[test]
    fn peripheral_summary_includes_advertised_services() {
        let text = format_peripheral_summary(&DebugPeripheralSummary {
            local_name: "yundrone-065333".to_string(),
            rssi: Some(-22),
            address: "AA:BB:CC:DD:EE:FF".to_string(),
            advertised_services: vec![protocol::commands::CMD_LINK_HEARTBEAT.to_string()],
        });

        assert!(text.contains("yundrone-065333"));
        assert!(text.contains("-22 dBm"));
        assert!(text.contains("AA:BB:CC:DD:EE:FF"));
    }
}
