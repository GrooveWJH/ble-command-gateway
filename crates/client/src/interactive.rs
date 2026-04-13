use anyhow::Result;
use client::{prepare_request, BleClient, BleSession, ScanCandidateInfo, ScannedDevice};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::Table;
use inquire::{Password, Select, Text};
use protocol::requests::CommandPayload;
use protocol::responses::{StatusResponseData, WifiScanResponseData};
use std::fmt;
use tracing::info;

use crate::cli_text::Lang;
use crate::Args;

pub(crate) async fn run_cli(args: Args) -> Result<()> {
    let lang = Lang::from_cli_arg(&args.lang);
    println!(">>> BLE Command Gateway Interactive CLI <<<");
    println!("{}", lang.scan_header(&args.target, args.timeout));

    let client = BleClient::new().await?;
    let device = scan_and_select_device(&client, &lang, &args.target, args.timeout).await?;

    println!("{}", lang.t("found_conn").replace("{}", &device.info.name));
    let mut session = client.connect_session(device).await?;
    println!("{}", lang.t("handshake_ok"));

    run_menu_loop(&mut session, lang).await
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
    let candidates = client.scan_candidates(target, timeout).await?;

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

async fn run_menu_loop(session: &mut BleSession, lang: Lang) -> Result<()> {
    loop {
        match prompt_menu_action(&lang)? {
            MenuAction::Exit => {
                println!("Goodbye!");
                return Ok(());
            }
            MenuAction::Status => run_status(session).await?,
            MenuAction::WifiScan => run_wifi_scan(session).await?,
            MenuAction::Provision => run_provision(session, &lang).await?,
        }
    }
}

async fn run_status(session: &mut BleSession) -> Result<()> {
    println!(">> Sending Status Command...");
    let response = execute_request(session, CommandPayload::Status, 10).await?;
    let data: StatusResponseData = response.decode_data()?;

    let mut table = Table::new();
    table.apply_modifier(UTF8_ROUND_CORNERS);
    table.set_header(vec!["Metric", "Value"]);
    table.add_row(vec!["Hostname", &data.hostname]);
    table.add_row(vec!["System", &data.system]);
    table.add_row(vec!["User", &data.user]);
    table.add_row(vec![
        "Network",
        data.network.as_deref().unwrap_or("Not connected"),
    ]);
    table.add_row(vec!["IP", data.ip.as_deref().unwrap_or("Unavailable")]);
    println!("{table}");
    Ok(())
}

async fn run_wifi_scan(session: &mut BleSession) -> Result<()> {
    println!(">> Requesting Wi-Fi Scan...");
    let response = execute_request(session, CommandPayload::WifiScan { ifname: None }, 15).await?;
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

async fn run_provision(session: &mut BleSession, lang: &Lang) -> Result<()> {
    let ssid = Text::new(lang.t("prmpt_ssid")).prompt()?;
    let pwd = Password::new(lang.t("prmpt_pwd")).prompt()?;
    let response = execute_request(
        session,
        CommandPayload::Provision {
            ssid,
            pwd: (!pwd.is_empty()).then_some(pwd),
        },
        30,
    )
    .await?;

    println!("{}", response.text);
    Ok(())
}

async fn execute_request(
    session: &mut BleSession,
    payload: CommandPayload,
    timeout_secs: u64,
) -> Result<protocol::CommandResponse> {
    let request = prepare_request(payload)?;
    session.send_request(&request).await?;
    let response = session.next_response(timeout_secs).await?;
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
            lang.t("opt_exit"),
        ],
    )
    .prompt()?;

    Ok(match selected {
        value if value == lang.t("opt_stat") => MenuAction::Status,
        value if value == lang.t("opt_scan") => MenuAction::WifiScan,
        value if value == lang.t("opt_prov") => MenuAction::Provision,
        _ => MenuAction::Exit,
    })
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
    Exit,
}

#[cfg(test)]
mod tests {
    use super::format_scan_candidate_label;

    #[test]
    fn format_scan_candidate_label_shows_name_and_signal() {
        let label = format_scan_candidate_label(&client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-A7".to_string(),
            rssi: Some(-41),
        });

        assert_eq!(label, "Yundrone_UAV-15-19-A7 (-41 dBm)");
    }

    #[test]
    fn format_scan_candidate_label_handles_missing_signal() {
        let label = format_scan_candidate_label(&client::ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-UNK".to_string(),
            rssi: None,
        });

        assert_eq!(label, "Yundrone_UAV-15-19-UNK (RSSI unknown)");
    }
}
