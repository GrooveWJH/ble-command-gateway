use clap::{
    builder::styling::{AnsiColor, Effects, Styles},
    ColorChoice, Parser,
};

const CLI_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Yellow.on_default())
    .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
    .valid(AnsiColor::Green.on_default())
    .invalid(AnsiColor::Yellow.on_default());

const SERVER_EXAMPLES: &str = "\
Examples:
  sudo yundrone-ble-server
  sudo yundrone-ble-server --name-prefix yundrone
  sudo yundrone-ble-server --adapter hci0 --backend auto
  YUNDRONE_DEVICE_PREFIX=custom sudo -E yundrone-ble-server
";

#[derive(Clone, Debug, Parser, PartialEq, Eq)]
#[command(
    author,
    version,
    name = "yundrone-ble-server",
    about = "YunDrone BLE gateway server",
    long_about = "YunDrone BLE gateway server.\n\nRun the Linux BLE peripheral that exposes the YunDrone command service, advertises a stable local name, and executes Wi-Fi provisioning and diagnostics through NetworkManager.",
    next_line_help = true,
    color = ColorChoice::Always,
    styles = CLI_STYLES,
    after_help = SERVER_EXAMPLES
)]
pub struct ServerArgs {
    #[arg(
        long,
        env = "YUNDRONE_DEVICE_PREFIX",
        value_name = "PREFIX",
        default_value = protocol::config::DEFAULT_DEVICE_NAME,
        value_parser = parse_name_prefix,
        help = "Public BLE name prefix; allowed characters are lowercase letters, digits, and '-'"
    )]
    pub name_prefix: String,

    #[arg(
        long,
        env = "YUNDRONE_BLE_ADAPTER",
        value_name = "HCI",
        value_parser = parse_adapter_name,
        help = "Bluetooth adapter name, for example hci0; defaults to the BlueZ default adapter"
    )]
    pub adapter: Option<String>,

    #[arg(
        long,
        env = "YUNDRONE_BLE_ADV_BACKEND",
        value_name = "BACKEND",
        default_value = "auto",
        value_parser = crate::advertising_backend::AdvertisingBackend::parse,
        help = "Advertising backend: auto, bluez-dbus, or legacy-hci"
    )]
    pub backend: crate::advertising_backend::AdvertisingBackend,
}

pub fn parse_args() -> ServerArgs {
    ServerArgs::parse()
}

fn parse_name_prefix(value: &str) -> Result<String, String> {
    crate::device_identity::validate_name_prefix(value)?;
    Ok(value.to_string())
}

fn parse_adapter_name(value: &str) -> Result<String, String> {
    let valid = value
        .strip_prefix("hci")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()));
    valid
        .then(|| value.to_string())
        .ok_or_else(|| "蓝牙适配器必须是 hci0、hci1 等格式".to_string())
}

pub fn device_prefix_from_args(args: &ServerArgs) -> anyhow::Result<String> {
    crate::device_identity::validate_name_prefix(&args.name_prefix).map_err(anyhow::Error::msg)?;
    Ok(args.name_prefix.clone())
}

#[cfg(test)]
mod tests {
    use super::{device_prefix_from_args, ServerArgs};
    use clap::{ColorChoice, CommandFactory, Parser};

    #[test]
    fn accepts_valid_custom_prefix() {
        let args = ServerArgs {
            name_prefix: "custom-drone".to_string(),
            adapter: None,
            backend: crate::advertising_backend::AdvertisingBackend::Auto,
        };

        assert_eq!(device_prefix_from_args(&args).unwrap(), "custom-drone");
    }

    #[test]
    fn rejects_invalid_custom_prefix() {
        let args = ServerArgs {
            name_prefix: "Custom_Drone".to_string(),
            adapter: None,
            backend: crate::advertising_backend::AdvertisingBackend::Auto,
        };

        assert!(device_prefix_from_args(&args).is_err());
    }

    #[test]
    fn cli_rejects_invalid_prefix_with_usage_error() {
        let err = ServerArgs::try_parse_from(["server", "--name-prefix", "Custom_Drone"])
            .expect_err("invalid prefix should be rejected by clap");

        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
        assert!(err.to_string().contains("lowercase ASCII"));
    }

    #[test]
    fn cli_prefix_overrides_default() {
        let args = ServerArgs::parse_from(["server", "--name-prefix", "custom"]);

        assert_eq!(args.name_prefix, "custom");
        assert_eq!(
            args.backend,
            crate::advertising_backend::AdvertisingBackend::Auto
        );
    }

    #[test]
    fn default_prefix_is_yundrone() {
        let args = ServerArgs::parse_from(["server"]);

        assert_eq!(args.name_prefix, protocol::config::DEFAULT_DEVICE_NAME);
        assert_eq!(args.adapter, None);
    }

    #[test]
    fn cli_accepts_adapter_and_backend() {
        let args =
            ServerArgs::parse_from(["server", "--adapter", "hci1", "--backend", "legacy-hci"]);

        assert_eq!(args.adapter.as_deref(), Some("hci1"));
        assert_eq!(
            args.backend,
            crate::advertising_backend::AdvertisingBackend::LegacyHci
        );
    }

    #[test]
    fn cli_rejects_unknown_backend() {
        let err = ServerArgs::try_parse_from(["server", "--backend", "nope"])
            .expect_err("unknown backend should be rejected");
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn cli_rejects_invalid_adapter_name() {
        let err = ServerArgs::try_parse_from(["server", "--adapter", "blue0"])
            .expect_err("invalid adapter should be rejected");
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn server_help_is_typer_like_and_example_driven() {
        let mut command = ServerArgs::command();
        let help = command.render_long_help().to_string();

        assert_eq!(command.get_color(), ColorChoice::Always);
        assert!(help.contains("YunDrone BLE gateway server"));
        assert!(help.contains("Examples:"));
        assert!(help.contains("yundrone-ble-server --name-prefix yundrone"));
        assert!(help.contains("YUNDRONE_DEVICE_PREFIX=custom"));
    }
}
