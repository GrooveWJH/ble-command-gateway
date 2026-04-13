mod cli_text;
mod interactive;

use anyhow::Result;
use clap::Parser;

const CLI_RUNTIME: platform_runtime::AppRuntime = platform_runtime::AppRuntime {
    bundle_name: "YunDrone BLE Client.app",
    executable_name: "client",
    info_plist: include_bytes!("../macos/Info.plist"),
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    #[arg(short, long, default_value = "Yundrone_UAV")]
    pub(crate) target: String,

    #[arg(long, default_value_t = 30)]
    pub(crate) timeout: u64,

    #[arg(long, default_value = "zh")]
    pub(crate) lang: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let raw_args = std::env::args().skip(1).collect::<Vec<_>>();
    if platform_runtime::prepare_cli_runtime(&CLI_RUNTIME, &raw_args)?
        == platform_runtime::RuntimeLaunchOutcome::Relaunched
    {
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    interactive::run_cli(Args::parse()).await
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn macos_bundle_declares_bluetooth_usage_description() {
        let plist_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("macos/Info.plist");
        let plist = std::fs::read_to_string(&plist_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", plist_path.display()));

        assert!(plist.contains("CFBundleIdentifier"));
        assert!(plist.contains("CFBundleExecutable"));
        assert!(plist.contains("CFBundlePackageType"));
        assert!(plist.contains("NSBluetoothAlwaysUsageDescription"));
        assert!(plist.contains("Bluetooth"));
    }
}
