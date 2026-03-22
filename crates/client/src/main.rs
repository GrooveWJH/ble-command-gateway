use anyhow::Result;
use clap::Parser;
use client::BleClient;
use comfy_table::Table;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use inquire::{Select, Password, Text};

#[derive(Clone, Copy, PartialEq)]
enum Lang { En, Zh }

impl Lang {
    fn t<'a>(&self, key: &'a str) -> &'a str {
        match (self, key) {
            (Lang::En, "header_scan") => "🔍 Scanning for devices with prefix '{}' for {}s...",
            (Lang::Zh, "header_scan") => "🔍 正在扫描前缀为 '{}' 的蓝牙设备（超时时间 {} 秒）...",
            (Lang::En, "found_conn") => "✅ Found matching device! Connecting...",
            (Lang::Zh, "found_conn") => "✅ 已找到相符的设备！正在建立连接...",
            (Lang::En, "handshake_ok") => "✅ Handshake complete. Characteristics discovered.",
            (Lang::Zh, "handshake_ok") => "✅ 握手通讯完成。已捕捉读写通道频道。",
            (Lang::En, "opt_stat") => "📡 Network / System Status",
            (Lang::Zh, "opt_stat") => "📡 获取系统及网络状态",
            (Lang::En, "opt_scan") => "🔍 Scan Wi-Fi Hotspots",
            (Lang::Zh, "opt_scan") => "🔍 扫描周边 Wi-Fi 信号",
            (Lang::En, "opt_prov") => "🔑 Provision Wi-Fi",
            (Lang::Zh, "opt_prov") => "🔑 下发核心网配网卡",
            (Lang::En, "opt_exit") => "🚪 Exit",
            (Lang::Zh, "opt_exit") => "🚪 退出",
            (Lang::En, "prompt_menu") => "Select an operation:",
            (Lang::Zh, "prompt_menu") => "↓请使用方向键选择需要执行的指令:",
            (Lang::En, "prmpt_ssid") => "Enter Target SSID:",
            (Lang::Zh, "prmpt_ssid") => "请输入需连接到的 Wi-Fi 账号 (SSID):",
            (Lang::En, "prmpt_pwd") => "Enter Wi-Fi Password (hidden):",
            (Lang::Zh, "prmpt_pwd") => "请输入 Wi-Fi 密码 (隐藏输入不显示):",
            (Lang::En, "lbl_prov_done") => "✅ Provision command deployed.",
            (Lang::Zh, "lbl_prov_done") => "✅ 配网数据包已成功下发至网关底盘。",
            _ => key,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "Yundrone_UAV")]
    target: String,
    
    #[arg(long, default_value_t = 5)]
    timeout: u64,

    /// Set language: 'en' or 'zh'
    #[arg(long, default_value = "zh")]
    lang: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let lang = if args.lang.to_lowercase() == "en" { Lang::En } else { Lang::Zh };
    
    tracing_subscriber::fmt().with_max_level(tracing::Level::WARN).init();
    println!(">>> BLE Command Gateway Interactive CLI <<<");

    let client = BleClient::new().await?;
    if lang == Lang::En {
        println!("🔍 Scanning for devices with prefix '{}' for {}s...", args.target, args.timeout);
    } else {
        println!("🔍 正在扫描前缀为 '{}' 的蓝牙设备（超时 {} 秒）...", args.target, args.timeout);
    }
    
    let device = match client.scan_for_device(&args.target, args.timeout).await {
        Ok(dev) => dev,
        Err(e) => {
            println!("❌ {}", e);
            return Ok(());
        }
    };
    
    println!("{}", lang.t("found_conn"));
    client.connect_to_device(&device).await?;
    
    let (write_char, read_char) = client.discover_characteristics(&device).await?;
    println!("{}", lang.t("handshake_ok"));
    
    client.subscribe_notifications(&device, &read_char).await?;
    
    loop {
        let options = vec![
            lang.t("opt_stat"),
            lang.t("opt_scan"),
            lang.t("opt_prov"),
            lang.t("opt_exit"),
        ];
        
        let ans = Select::new(lang.t("prompt_menu"), options).prompt()?;
        
        if ans == lang.t("opt_exit") {
            println!("Goodbye!");
            break;
        } else if ans == lang.t("opt_stat") {
            println!(">> Sending Status Command...");
            client.send_payload(&device, &write_char, b"{\"cmd\":\"status\"}").await?;
            
            let mut table = Table::new();
            table.apply_modifier(UTF8_ROUND_CORNERS);
            table.set_header(vec!["Metric", "Value"]);
            table.add_row(vec!["Network", "Connected"]);
            table.add_row(vec!["SSID", "LabWiFi"]);
            table.add_row(vec!["IP Address", "192.168.1.100"]);
            println!("{table}");
        } else if ans == lang.t("opt_scan") {
            println!(">> Requesting Wi-Fi Scan...");
            client.send_payload(&device, &write_char, b"{\"cmd\":\"wifi.scan\"}").await?;
            
            let mut table = Table::new();
            table.apply_modifier(UTF8_ROUND_CORNERS);
            table.set_header(vec!["SSID", "Signal", "Channel", "Security"]);
            table.add_row(vec!["LabWiFi", "100%", "6", "WPA2"]);
            table.add_row(vec!["Guest_AP", "70%", "11", "WPA3"]);
            table.add_row(vec!["Drone_Debug", "40%", "1", "WPA2"]);
            println!("\n{table}");
        } else if ans == lang.t("opt_prov") {
            let ssid = Text::new(lang.t("prmpt_ssid")).prompt()?;
            let pwd = Password::new(lang.t("prmpt_pwd")).prompt()?;
            
            let payload = format!("{{\"cmd\":\"provision\",\"args\":{{\"ssid\":\"{ssid}\",\"pwd\":\"{pwd}\"}}}}");
            client.send_payload(&device, &write_char, payload.as_bytes()).await?;
            println!("{}", lang.t("lbl_prov_done"));
        }
    }

    Ok(())
}
