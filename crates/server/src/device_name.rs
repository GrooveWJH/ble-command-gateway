pub const NULL_DEVICE_SERIAL: &str = "null";

pub fn build_device_name_from_mac(base_prefix: &str, address: Option<[u8; 6]>) -> String {
    let serial = address
        .filter(is_usable_mac_address)
        .map(serial_from_mac)
        .unwrap_or_else(|| NULL_DEVICE_SERIAL.to_string());
    format!("{base_prefix}-{serial}")
}

pub fn serial_from_mac(address: [u8; 6]) -> String {
    format!("{:02x}{:02x}{:02x}", address[3], address[4], address[5])
}

pub fn is_usable_mac_address(address: &[u8; 6]) -> bool {
    *address != [0; 6] && *address != [0xff; 6]
}

#[cfg(test)]
mod tests {
    use super::{build_device_name_from_mac, is_usable_mac_address, serial_from_mac};

    #[test]
    fn device_name_uses_last_six_mac_digits() {
        let address = [0xdc, 0xa6, 0x32, 0x12, 0xab, 0xcd];

        assert_eq!(serial_from_mac(address), "12abcd");
        assert_eq!(
            build_device_name_from_mac("yundrone", Some(address)),
            "yundrone-12abcd"
        );
    }

    #[test]
    fn device_name_respects_custom_prefix() {
        let address = [0x00, 0x11, 0x22, 0xa1, 0xb2, 0xc3];

        assert_eq!(
            build_device_name_from_mac("edge", Some(address)),
            "edge-a1b2c3"
        );
    }

    #[test]
    fn missing_or_invalid_mac_uses_null_suffix() {
        assert_eq!(
            build_device_name_from_mac("yundrone", None),
            "yundrone-null"
        );
        assert_eq!(
            build_device_name_from_mac("yundrone", Some([0; 6])),
            "yundrone-null"
        );
        assert_eq!(
            build_device_name_from_mac("yundrone", Some([0xff; 6])),
            "yundrone-null"
        );
        assert!(!is_usable_mac_address(&[0; 6]));
        assert!(!is_usable_mac_address(&[0xff; 6]));
    }
}
