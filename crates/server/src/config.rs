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
}

pub fn parse_args() -> ServerArgs {
    ServerArgs::parse()
}

fn parse_name_prefix(value: &str) -> Result<String, String> {
    crate::device_identity::validate_name_prefix(value)?;
    Ok(value.to_string())
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
        };

        assert_eq!(device_prefix_from_args(&args).unwrap(), "custom-drone");
    }

    #[test]
    fn rejects_invalid_custom_prefix() {
        let args = ServerArgs {
            name_prefix: "Custom_Drone".to_string(),
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
    }

    #[test]
    fn default_prefix_is_yundrone() {
        let args = ServerArgs::parse_from(["server"]);

        assert_eq!(args.name_prefix, protocol::config::DEFAULT_DEVICE_NAME);
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
