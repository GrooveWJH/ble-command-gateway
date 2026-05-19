#[cfg(target_os = "linux")]
use anyhow::{anyhow, bail, Context};
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::time::Duration;
#[cfg(target_os = "linux")]
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub struct LegacyAdvertisingSession {
    adapter_name: String,
}

#[cfg(target_os = "linux")]
pub async fn start_legacy_advertising(
    adapter_name: &str,
    identity_name: &str,
    service_uuid: Uuid,
    interval: crate::advertising::AdvertisingInterval,
) -> anyhow::Result<LegacyAdvertisingSession> {
    let _ = set_legacy_enabled(adapter_name, false);
    apply_legacy_interval(adapter_name, interval)?;
    set_legacy_adv_data(adapter_name, &build_primary_payload(identity_name)?)?;
    set_legacy_scan_response(adapter_name, &build_scan_response_payload(service_uuid)?)?;
    set_legacy_enabled(adapter_name, true)?;
    Ok(LegacyAdvertisingSession {
        adapter_name: adapter_name.to_string(),
    })
}

#[cfg(target_os = "linux")]
impl LegacyAdvertisingSession {
    pub async fn stop(self) -> anyhow::Result<()> {
        set_legacy_enabled(&self.adapter_name, false)
    }
}

#[cfg(target_os = "linux")]
fn apply_legacy_interval(
    adapter_name: &str,
    interval: crate::advertising::AdvertisingInterval,
) -> anyhow::Result<()> {
    let mut args = vec![
        "-i".to_string(),
        adapter_name.to_string(),
        "cmd".to_string(),
        "0x08".to_string(),
        "0x0006".to_string(),
    ];
    args.extend(format_interval_args(interval));
    run_hcitool(&args).context("set legacy advertising interval")
}

#[cfg(target_os = "linux")]
fn set_legacy_adv_data(adapter_name: &str, payload: &[u8]) -> anyhow::Result<()> {
    let mut args = vec![
        "-i".to_string(),
        adapter_name.to_string(),
        "cmd".to_string(),
        "0x08".to_string(),
        "0x0008".to_string(),
    ];
    args.extend(format_data_args(payload)?);
    run_hcitool(&args).context("set legacy advertising data")
}

#[cfg(target_os = "linux")]
fn set_legacy_scan_response(adapter_name: &str, payload: &[u8]) -> anyhow::Result<()> {
    let mut args = vec![
        "-i".to_string(),
        adapter_name.to_string(),
        "cmd".to_string(),
        "0x08".to_string(),
        "0x0009".to_string(),
    ];
    args.extend(format_data_args(payload)?);
    run_hcitool(&args).context("set legacy scan response")
}

#[cfg(target_os = "linux")]
fn set_legacy_enabled(adapter_name: &str, enabled: bool) -> anyhow::Result<()> {
    let flag = if enabled { "01" } else { "00" };
    run_hcitool(&[
        "-i".to_string(),
        adapter_name.to_string(),
        "cmd".to_string(),
        "0x08".to_string(),
        "0x000A".to_string(),
        flag.to_string(),
    ])
    .with_context(|| format!("set legacy advertising enabled={enabled}"))
}

#[cfg(target_os = "linux")]
fn run_hcitool(args: &[String]) -> anyhow::Result<()> {
    let output = Command::new("hcitool")
        .args(args)
        .output()
        .context("spawn hcitool")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(anyhow!(
        "hcitool failed: status={} stdout={} stderr={}",
        output.status,
        stdout.trim(),
        stderr.trim()
    ))
}

#[cfg(target_os = "linux")]
fn format_interval_args(interval: crate::advertising::AdvertisingInterval) -> Vec<String> {
    let [min_lo, min_hi] = interval_units(interval.min).to_le_bytes();
    let [max_lo, max_hi] = interval_units(interval.max).to_le_bytes();
    [
        min_lo, min_hi, max_lo, max_hi, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07,
        0x00,
    ]
    .into_iter()
    .map(|byte| format!("{byte:02X}"))
    .collect()
}

#[cfg(target_os = "linux")]
fn format_data_args(payload: &[u8]) -> anyhow::Result<Vec<String>> {
    let len = u8::try_from(payload.len()).context("payload too large for HCI command")?;
    if payload.len() > 31 {
        bail!("legacy advertising data exceeds 31 bytes");
    }
    let mut args = vec![format!("{len:02X}")];
    args.extend(payload.iter().map(|byte| format!("{byte:02X}")));
    args.extend(std::iter::repeat_n("00".to_string(), 31 - payload.len()));
    Ok(args)
}

#[cfg(target_os = "linux")]
fn interval_units(duration: Duration) -> u16 {
    let micros = duration.as_micros() as u64;
    let rounded = (micros + 312) / 625;
    rounded.clamp(0x0020, 0x4000) as u16
}

#[cfg(target_os = "linux")]
fn build_primary_payload(identity_name: &str) -> anyhow::Result<Vec<u8>> {
    let name = identity_name.as_bytes();
    let mut payload = vec![0x02, 0x01, 0x06];
    payload.push(
        u8::try_from(name.len() + 1).context("identity name too long for advertising payload")?,
    );
    payload.push(0x09);
    payload.extend_from_slice(name);
    if payload.len() > 31 {
        bail!("primary advertising payload exceeds 31 bytes");
    }
    Ok(payload)
}

#[cfg(target_os = "linux")]
fn build_scan_response_payload(service_uuid: Uuid) -> anyhow::Result<Vec<u8>> {
    let mut payload = vec![0x11, 0x07];
    payload.extend_from_slice(&service_uuid.to_bytes_le());
    if payload.len() > 31 {
        bail!("scan response payload exceeds 31 bytes");
    }
    Ok(payload)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{
        build_primary_payload, build_scan_response_payload, format_data_args, format_interval_args,
    };
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn formats_legacy_interval_command_for_twenty_ms() {
        let args = format_interval_args(crate::advertising::AdvertisingInterval {
            min: Duration::from_millis(20),
            max: Duration::from_millis(20),
        });

        assert_eq!(
            args,
            vec![
                "20", "00", "20", "00", "00", "00", "00", "00", "00", "00", "00", "00", "00", "07",
                "00"
            ]
        );
    }

    #[test]
    fn formats_legacy_interval_command_for_steady_211_25_ms() {
        let args = format_interval_args(crate::advertising::AdvertisingInterval {
            min: Duration::from_micros(211_250),
            max: Duration::from_micros(211_250),
        });

        assert_eq!(
            args,
            vec![
                "52", "01", "52", "01", "00", "00", "00", "00", "00", "00", "00", "00", "00", "07",
                "00"
            ]
        );
    }

    #[test]
    fn primary_payload_carries_single_identity_name() {
        let payload = build_primary_payload("yundrone-ytcwln").unwrap();

        assert_eq!(&payload[..3], &[0x02, 0x01, 0x06]);
        assert_eq!(payload[3], 16);
        assert_eq!(payload[4], 0x09);
        assert_eq!(&payload[5..], b"yundrone-ytcwln");
        assert_eq!(payload.len(), 20);
    }

    #[test]
    fn scan_response_payload_carries_uart_service_uuid() {
        let payload = build_scan_response_payload(
            Uuid::parse_str("6E400001-B5A3-F393-E0A9-E50E24DCCA9E").unwrap(),
        )
        .unwrap();

        assert_eq!(payload.len(), 18);
        assert_eq!(&payload[..2], &[0x11, 0x07]);
    }

    #[test]
    fn formats_hci_data_args_with_length_prefix() {
        let args = format_data_args(&[0x02, 0x01, 0x06]).unwrap();

        assert_eq!(&args[..4], &["03", "02", "01", "06"]);
        assert_eq!(args.len(), 32);
        assert!(args[4..].iter().all(|value| value == "00"));
    }
}
