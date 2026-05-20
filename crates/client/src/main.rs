mod cli_text;
mod debug_ble;
mod debug_qos;
mod interactive;
mod profiles;

use anyhow::Result;
use clap::{
    builder::styling::{AnsiColor, Effects, Styles},
    ColorChoice, Parser, Subcommand,
};

#[cfg(target_os = "macos")]
const MACOS_INFO_PLIST: &[u8] = include_bytes!("../macos/Info.plist");

#[cfg(target_os = "macos")]
#[used]
#[link_section = "__TEXT,__info_plist"]
static EMBEDDED_MACOS_INFO_PLIST: [u8; MACOS_INFO_PLIST.len()] =
    *include_bytes!("../macos/Info.plist");

const CLI_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Yellow.on_default())
    .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
    .valid(AnsiColor::Green.on_default())
    .invalid(AnsiColor::Yellow.on_default());

const CLIENT_EXAMPLES: &str = "\
Examples:
  yundrone-ble-client interactive --lang zh
  yundrone-ble-client interactive --target yundrone --timeout 30
  yundrone-ble-client interactive --verbose
  yundrone-ble-client debug-ble --target yundrone --trace-chunks
  yundrone-ble-client debug-ble --target yundrone --output /tmp/yundrone-ble-debug.log
";

const INTERACTIVE_EXAMPLES: &str = "\
Examples:
  yundrone-ble-client interactive --lang zh
  yundrone-ble-client interactive --target yundrone --timeout 30 --lang en
  yundrone-ble-client interactive --verbose
  yundrone-ble-client interactive --verbose --verbose-unsafe-raw
";

const DEBUG_BLE_EXAMPLES: &str = "\
Examples:
  yundrone-ble-client debug-ble --target yundrone
  yundrone-ble-client debug-ble --target yundrone --trace-chunks
  yundrone-ble-client debug-ble --target yundrone --output /tmp/yundrone-ble-debug.log
";

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    name = "yundrone-ble-client",
    about = "YunDrone BLE provisioning and diagnostics client",
    long_about = "YunDrone BLE provisioning and diagnostics client.\n\nConnect to a YunDrone BLE gateway, provision Wi-Fi, inspect device status, and debug BLE links from the terminal.",
    arg_required_else_help = true,
    next_line_help = true,
    color = ColorChoice::Always,
    styles = CLI_STYLES,
    after_help = CLIENT_EXAMPLES
)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Run the interactive provisioning and diagnostics menu.
    #[command(
        about = "Open the interactive provisioning and diagnostics menu",
        long_about = "Scan for YunDrone BLE gateways, connect to a selected device, then run provisioning, status, and Wi-Fi profile actions from an interactive terminal menu.",
        next_line_help = true,
        after_help = INTERACTIVE_EXAMPLES
    )]
    Interactive(InteractiveArgs),

    /// Run a step-by-step BLE link diagnostic against a gateway.
    #[command(
        about = "Debug scan, connection, GATT discovery, and response frames",
        long_about = "Run a guided BLE link diagnostic: scan for a matching gateway, connect, discover UART GATT services, subscribe to notifications, send test commands, and optionally trace response_json chunks.",
        next_line_help = true,
        after_help = DEBUG_BLE_EXAMPLES
    )]
    DebugBle(DebugBleArgs),
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct InteractiveArgs {
    #[arg(
        short,
        long,
        value_name = "PREFIX",
        default_value = protocol::config::DEFAULT_DEVICE_NAME,
        help = "BLE local-name prefix to scan for"
    )]
    pub(crate) target: String,

    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 30,
        help = "Maximum scan duration before giving up"
    )]
    pub(crate) timeout: u64,

    #[arg(
        long,
        value_name = "LANG",
        default_value = "zh",
        help = "Interface language: zh or en"
    )]
    pub(crate) lang: String,

    #[arg(
        long,
        help = "Print TX/RX raw frames, response_json chunks, assembled responses, and QoS ACKs; sensitive fields are redacted by default"
    )]
    pub(crate) verbose: bool,

    #[arg(
        long,
        requires = "verbose",
        help = "Do not redact sensitive fields in verbose raw payloads; this can expose Wi-Fi passwords"
    )]
    pub(crate) verbose_unsafe_raw: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct DebugBleArgs {
    #[arg(
        short,
        long,
        value_name = "PREFIX",
        default_value = protocol::config::DEFAULT_DEVICE_NAME,
        help = "BLE local-name prefix to scan for"
    )]
    pub(crate) target: String,

    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 30,
        help = "Maximum scan duration before giving up"
    )]
    pub(crate) timeout: u64,

    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 10,
        help = "Maximum time to wait for each response event"
    )]
    pub(crate) response_timeout: u64,

    #[arg(
        long,
        value_name = "PATH",
        help = "Write the full debug transcript to a file"
    )]
    pub(crate) output: Option<std::path::PathBuf>,

    #[arg(
        long,
        help = "Trace response_json chunks and the final assembled response"
    )]
    pub(crate) trace_chunks: bool,

    #[arg(long, help = "Trace QoS writes, chunk ACKs, and event ACKs")]
    pub(crate) trace_qos: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    let args = Args::parse();
    match &args.command {
        Command::Interactive(interactive_args) => {
            interactive::run_cli(interactive_args.clone()).await
        }
        Command::DebugBle(debug_args) => {
            debug_ble::run(
                &debug_args.target,
                debug_args.timeout,
                debug_args.response_timeout,
                debug_args.output.clone(),
                debug_args.trace_chunks,
                debug_args.trace_qos,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use clap::{ColorChoice, CommandFactory};

    #[test]
    fn root_help_is_typer_like_and_example_driven() {
        let mut command = Args::command();
        let help = command.render_long_help().to_string();

        assert_eq!(command.get_color(), ColorChoice::Always);
        assert!(help.contains("YunDrone BLE provisioning and diagnostics client"));
        assert!(help.contains("Examples:"));
        assert!(help.contains("yundrone-ble-client interactive --lang zh"));
        assert!(help.contains("yundrone-ble-client interactive --verbose"));
        assert!(help.contains("yundrone-ble-client debug-ble --target yundrone --trace-chunks"));
    }

    #[test]
    fn debug_help_explains_chunk_tracing() {
        let mut command = Args::command();
        let help = command
            .find_subcommand_mut("debug-ble")
            .expect("debug-ble subcommand")
            .render_long_help()
            .to_string();

        assert!(help.contains("Trace response_json chunks"));
        assert!(help.contains("Examples:"));
    }

    #[test]
    fn interactive_help_explains_verbose_raw_tracing() {
        let mut command = Args::command();
        let help = command
            .find_subcommand_mut("interactive")
            .expect("interactive subcommand")
            .render_long_help()
            .to_string();

        assert!(help.contains("--verbose"));
        assert!(help.contains("--verbose-unsafe-raw"));
        assert!(help.contains("sensitive fields are redacted by default"));
        assert!(help.contains("can expose Wi-Fi passwords"));
        assert!(help.contains("yundrone-ble-client interactive --verbose"));
    }
}
