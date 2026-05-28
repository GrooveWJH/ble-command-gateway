use anyhow::Result;
use client::{
    prepare_request, scan_state::merge_scanned_device, BleClient, BleSession, ScanCandidateInfo,
    ScannedDevice,
};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::Table;
use crossterm::event::{self, Event, KeyCode};
use inquire::{Password, Select, Text};
use protocol::requests::CommandPayload;
use protocol::responses::{StatusResponseData, WifiScanResponseData};
use std::{
    collections::BTreeMap,
    fmt,
    io::{self, IsTerminal, Write},
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};
use tracing::info;

use crate::cli_text::Lang;
use crate::InteractiveArgs;

pub(crate) async fn run_cli(args: InteractiveArgs) -> Result<()> {
    let lang = Lang::from_cli_arg(&args.lang);
    println!(">>> BLE Command Gateway Interactive CLI <<<");
    println!("{}", lang.scan_header(&args.target, args.timeout));

    let client = BleClient::new().await?;
    let device = scan_and_select_device(&client, &lang, &args.target, args.timeout).await?;

    println!("{}", lang.t("found_conn").replace("{}", &device.info.name));
    let mut session = client.connect_session(device).await?;
    println!("{}", lang.t("handshake_ok"));

    let trace = args
        .verbose
        .then(|| InteractiveTracePrinter::new(args.verbose_unsafe_raw));
    run_menu_loop(&mut session, lang, trace).await
}

fn format_scan_candidate_label(candidate: &ScanCandidateInfo) -> String {
    let signal = candidate
        .rssi
        .map(|value| format!("{value} dBm"))
        .unwrap_or_else(|| "RSSI unknown".to_string());
    format!("{} ({signal})", candidate.name)
}

async fn scan_and_select_device(
    client: &BleClient,
    lang: &Lang,
    target: &str,
    timeout: u64,
) -> Result<ScannedDevice> {
    let candidates = scan_candidates_dynamic(client, lang, target, timeout).await?;

    println!("{}", lang.t("scan_results"));
    for candidate in &candidates {
        println!("  - {}", format_scan_candidate_label(&candidate.info));
    }

    if candidates.len() == 1 {
        println!("{}", lang.t("single_match"));
        return Ok(candidates
            .into_iter()
            .next()
            .expect("single candidate exists"));
    }

    let selected = Select::new(lang.t("prompt_device"), candidate_choices(&candidates)).prompt()?;
    Ok(candidates
        .into_iter()
        .find(|candidate| candidate.info.name == selected.info.name)
        .expect("selected device should exist in candidate list"))
}

async fn scan_candidates_dynamic(
    client: &BleClient,
    lang: &Lang,
    target: &str,
    timeout: u64,
) -> Result<Vec<ScannedDevice>> {
    let (candidate_tx, mut candidate_rx) = mpsc::unbounded_channel();
    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let start = Instant::now();
    let mut candidates = BTreeMap::<String, ScannedDevice>::new();
    let mut renderer = ScanRenderer::new(io::stdout().is_terminal());
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut key_tick = tokio::time::interval(Duration::from_millis(100));
    key_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let scan_task = client.scan_candidates_live(
        target,
        timeout,
        &mut cancel_rx,
        |_| {},
        move |device| {
            let _ = candidate_tx.send(device);
        },
    );
    tokio::pin!(scan_task);

    let scan_result = loop {
        tokio::select! {
            result = &mut scan_task => {
                break result;
            }
            _ = key_tick.tick(), if !candidates.is_empty() && io::stdin().is_terminal() => {
                if enter_pressed()? {
                    let _ = cancel_tx.send(true);
                }
            }
            Some(device) = candidate_rx.recv() => {
                upsert_scan_candidate(&mut candidates, device);
                renderer.render(lang, target, timeout, start, &candidates, true)?;
            }
            _ = tick.tick() => {
                renderer.render(lang, target, timeout, start, &candidates, false)?;
            }
        }
    };

    while let Ok(device) = candidate_rx.try_recv() {
        upsert_scan_candidate(&mut candidates, device);
    }
    renderer.clear()?;

    let summary = scan_result?;
    let mut devices = candidates.into_values().collect::<Vec<_>>();
    devices.sort_by(|left, right| {
        right
            .info
            .rssi
            .unwrap_or(i16::MIN)
            .cmp(&left.info.rssi.unwrap_or(i16::MIN))
            .then_with(|| left.info.name.cmp(&right.info.name))
    });

    if devices.is_empty() {
        anyhow::bail!("Device '{}' not found after {}s scan", target, timeout);
    }

    if summary.cancelled {
        println!("{}", lang.t("scan_stopped_early"));
    }

    Ok(devices)
}

fn upsert_scan_candidate(candidates: &mut BTreeMap<String, ScannedDevice>, device: ScannedDevice) {
    candidates
        .entry(device.info.name.clone())
        .and_modify(|existing| {
            *existing = merge_scanned_device(existing.clone(), device.clone());
        })
        .or_insert(device);
}

fn enter_pressed() -> Result<bool> {
    if !event::poll(Duration::from_millis(0))? {
        return Ok(false);
    }
    Ok(matches!(
        event::read()?,
        Event::Key(key) if key.code == KeyCode::Enter
    ))
}

struct ScanRenderer {
    interactive: bool,
    rendered_lines: usize,
    last_log_second: Option<u64>,
}

impl ScanRenderer {
    fn new(interactive: bool) -> Self {
        Self {
            interactive,
            rendered_lines: 0,
            last_log_second: None,
        }
    }

    fn render(
        &mut self,
        lang: &Lang,
        target: &str,
        timeout: u64,
        start: Instant,
        candidates: &BTreeMap<String, ScannedDevice>,
        force: bool,
    ) -> Result<()> {
        static SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let elapsed = start.elapsed().as_secs();
        let remaining = timeout.saturating_sub(elapsed);
        let frame = ((start.elapsed().as_millis() / 250) as usize) % SPINNER.len();
        let lines = scan_status_lines(lang, SPINNER[frame], target, remaining, candidates);

        if !self.interactive {
            if force || elapsed.is_multiple_of(5) && self.last_log_second != Some(elapsed) {
                self.last_log_second = Some(elapsed);
                println!("{}", lines.first().map(String::as_str).unwrap_or_default());
            }
            return Ok(());
        }

        let mut out = io::stdout();
        if self.rendered_lines > 0 {
            write!(out, "\x1b[{}A", self.rendered_lines)?;
        }

        for line in &lines {
            write!(out, "\x1b[2K\r{line}\n")?;
        }

        for _ in lines.len()..self.rendered_lines {
            write!(out, "\x1b[2K\r\n")?;
        }

        if self.rendered_lines > lines.len() {
            write!(out, "\x1b[{}A", self.rendered_lines - lines.len())?;
        }

        self.rendered_lines = lines.len();
        out.flush()?;
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        if !self.interactive || self.rendered_lines == 0 {
            return Ok(());
        }

        let mut out = io::stdout();
        write!(out, "\x1b[{}A", self.rendered_lines)?;
        for _ in 0..self.rendered_lines {
            write!(out, "\x1b[2K\r\n")?;
        }
        write!(out, "\x1b[{}A", self.rendered_lines)?;
        out.flush()?;
        self.rendered_lines = 0;
        Ok(())
    }
}

fn scan_status_lines(
    lang: &Lang,
    spinner: &str,
    target: &str,
    remaining: u64,
    candidates: &BTreeMap<String, ScannedDevice>,
) -> Vec<String> {
    let mut lines = vec![
        lang.scan_live_status(spinner, target, remaining, candidates.len()),
        String::new(),
    ];

    if candidates.is_empty() {
        lines.push(lang.t("scan_waiting").to_string());
    } else {
        lines.extend(
            candidates
                .values()
                .map(|device| format!("  - {}", format_scan_candidate_label(&device.info))),
        );
        lines.push(String::new());
        lines.push(lang.t("scan_enter_to_select").to_string());
    }
    lines
}

async fn run_menu_loop(
    session: &mut BleSession,
    lang: Lang,
    trace: Option<InteractiveTracePrinter>,
) -> Result<()> {
    loop {
        match prompt_menu_action(&lang)? {
            MenuAction::Exit => {
                println!("Goodbye!");
                return Ok(());
            }
            MenuAction::Status => run_status(session, trace.as_ref()).await?,
            MenuAction::WifiScan => run_wifi_scan(session, trace.as_ref()).await?,
            MenuAction::Provision => run_provision(session, &lang, trace.as_ref()).await?,
            MenuAction::WifiProfiles => {
                crate::profiles::run_wifi_profiles(session, &lang, trace.as_ref()).await?
            }
        }
        pause_for_return(&lang)?;
    }
}

fn pause_for_return(lang: &Lang) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(());
    }

    print!("\n{}", lang.t("press_enter_return"));
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(())
}

async fn run_status(
    session: &mut BleSession,
    trace: Option<&InteractiveTracePrinter>,
) -> Result<()> {
    println!(">> Sending Status Command...");
    let response = execute_request(session, CommandPayload::SystemStatus, 10, trace).await?;
    let data: StatusResponseData = response.decode_data()?;

    let mut table = Table::new();
    table.apply_modifier(UTF8_ROUND_CORNERS);
    table.set_header(vec!["Metric", "Value"]);
    for (metric, value) in status_rows(&data) {
        table.add_row(vec![metric, value]);
    }
    println!("{table}");
    Ok(())
}

fn status_rows(data: &StatusResponseData) -> Vec<(String, String)> {
    let mut rows = vec![
        ("Device".to_string(), data.device_name.clone()),
        ("Hostname".to_string(), data.hostname.clone()),
        ("System".to_string(), data.system.clone()),
        ("User".to_string(), data.user.clone()),
        (
            "Network".to_string(),
            data.network
                .clone()
                .unwrap_or_else(|| "Not connected".to_string()),
        ),
        (
            "Preferred IP".to_string(),
            data.ip.clone().unwrap_or_else(|| "Unavailable".to_string()),
        ),
    ];

    if data.interfaces.is_empty() {
        rows.push(("Interfaces".to_string(), "none".to_string()));
    } else {
        rows.extend(
            data.interfaces
                .iter()
                .map(|interface| ("Interface".to_string(), interface.summary_line())),
        );
    }

    rows
}

async fn run_wifi_scan(
    session: &mut BleSession,
    trace: Option<&InteractiveTracePrinter>,
) -> Result<()> {
    println!(">> Requesting Wi-Fi Scan...");
    let response = execute_request(
        session,
        CommandPayload::WifiScan { ifname: None },
        15,
        trace,
    )
    .await?;
    let data: WifiScanResponseData = response.decode_data()?;

    let mut table = Table::new();
    table.apply_modifier(UTF8_ROUND_CORNERS);
    table.set_header(vec!["SSID", "Signal", "Channel", "Security"]);
    for network in data.networks {
        table.add_row(vec![
            &network.ssid,
            &format!("{}", network.signal),
            &network.channel,
            "-",
        ]);
    }
    println!("\n{table}");
    Ok(())
}

async fn run_provision(
    session: &mut BleSession,
    lang: &Lang,
    trace: Option<&InteractiveTracePrinter>,
) -> Result<()> {
    let ssid = Text::new(lang.t("prmpt_ssid")).prompt()?;
    let pwd = Password::new(lang.t("prmpt_pwd")).prompt()?;
    let response = execute_request(
        session,
        CommandPayload::WifiProvision {
            ssid,
            pwd: (!pwd.is_empty()).then_some(pwd),
        },
        30,
        trace,
    )
    .await?;

    println!("{}", response.text);
    Ok(())
}

pub(crate) async fn execute_request(
    session: &mut BleSession,
    payload: CommandPayload,
    timeout_secs: u64,
    trace: Option<&InteractiveTracePrinter>,
) -> Result<protocol::CommandResponse> {
    let request = prepare_request(payload)?;
    let response = if let Some(trace) = trace {
        let trace_options = trace.options();
        session
            .run_request_until_final_traced(
                &request,
                timeout_secs,
                trace_options,
                Some(trace.callback()),
                |event| {
                    if !event.final_flag {
                        println!(".. {}", event.text);
                    }
                },
            )
            .await?
    } else {
        session
            .run_request_until_final(&request, timeout_secs, |event| {
                if !event.final_flag {
                    println!(".. {}", event.text);
                }
            })
            .await?
    };
    info!(
        device_name = %session.device_name(),
        rssi = ?session.device_rssi(),
        cmd = %request.request.payload.command_name(),
        request_id = %request.request.id,
        response_id = %response.id,
        "cli.command.completed"
    );
    Ok(response)
}

pub(crate) struct InteractiveTracePrinter {
    options: client::trace::TraceOptions,
}

impl InteractiveTracePrinter {
    fn new(unsafe_raw: bool) -> Self {
        let options = if unsafe_raw {
            client::trace::TraceOptions::unsafe_raw()
        } else {
            client::trace::TraceOptions::safe()
        };
        Self { options }
    }

    fn options(&self) -> client::trace::TraceOptions {
        self.options
    }

    fn callback(&self) -> client::trace::TraceCallback {
        client::trace::printing_callback()
    }
}

fn candidate_choices(candidates: &[ScannedDevice]) -> Vec<CandidateChoice> {
    candidates
        .iter()
        .map(|candidate| CandidateChoice {
            info: candidate.info.clone(),
        })
        .collect()
}

fn prompt_menu_action(lang: &Lang) -> Result<MenuAction> {
    let selected = Select::new(
        lang.t("prompt_menu"),
        vec![
            lang.t("opt_stat"),
            lang.t("opt_scan"),
            lang.t("opt_prov"),
            lang.t("opt_profiles"),
            lang.t("opt_exit"),
        ],
    )
    .prompt()?;

    Ok(match selected {
        value if value == lang.t("opt_stat") => MenuAction::Status,
        value if value == lang.t("opt_scan") => MenuAction::WifiScan,
        value if value == lang.t("opt_prov") => MenuAction::Provision,
        value if value == lang.t("opt_profiles") => MenuAction::WifiProfiles,
        _ => MenuAction::Exit,
    })
}

#[cfg(test)]
mod tests {
    use super::{format_scan_candidate_label, status_rows};

    #[test]
    fn status_rows_include_preferred_ip_and_interfaces() {
        let rows = status_rows(&protocol::responses::StatusResponseData {
            device_name: "yundrone-ytcwln".to_string(),
            hostname: "edge-gateway".to_string(),
            system: "Linux 6.1".to_string(),
            user: "yundrone".to_string(),
            network: Some("LabWiFi".to_string()),
            ip: Some("192.0.2.2".to_string()),
            interfaces: vec![
                protocol::responses::StatusInterfaceIpv4 {
                    ifname: "wlan0".to_string(),
                    kind: protocol::responses::StatusInterfaceKind::Wifi,
                    ipv4: "192.0.2.2".to_string(),
                },
                protocol::responses::StatusInterfaceIpv4 {
                    ifname: "eth0".to_string(),
                    kind: protocol::responses::StatusInterfaceKind::Ethernet,
                    ipv4: "198.51.100.9".to_string(),
                },
            ],
        });

        assert!(rows.contains(&("Preferred IP".to_string(), "192.0.2.2".to_string())));
        assert!(rows.contains(&(
            "Interface".to_string(),
            "wlan0 [wifi] -> 192.0.2.2".to_string()
        )));
        assert!(rows.contains(&(
            "Interface".to_string(),
            "eth0 [ethernet] -> 198.51.100.9".to_string()
        )));
    }

    #[test]
    fn format_scan_candidate_label_shows_name_and_signal() {
        let label = format_scan_candidate_label(&client::ScanCandidateInfo {
            name: "yundrone-00a700".to_string(),
            rssi: Some(-41),
        });

        assert_eq!(label, "yundrone-00a700 (-41 dBm)");
    }

    #[test]
    fn format_scan_candidate_label_handles_missing_signal() {
        let label = format_scan_candidate_label(&client::ScanCandidateInfo {
            name: "yundrone-000000".to_string(),
            rssi: None,
        });

        assert_eq!(label, "yundrone-000000 (RSSI unknown)");
    }
}

#[derive(Clone)]
struct CandidateChoice {
    info: ScanCandidateInfo,
}

impl fmt::Display for CandidateChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_scan_candidate_label(&self.info))
    }
}

enum MenuAction {
    Status,
    WifiScan,
    Provision,
    WifiProfiles,
    Exit,
}
