use super::provision_panel::aggregate_wifi_networks;
use crate::i18n::Lang;

fn network(ssid: &str, channel: &str, signal: i32) -> protocol::responses::WifiNetwork {
    protocol::responses::WifiNetwork {
        ssid: ssid.to_string(),
        channel: channel.to_string(),
        signal,
    }
}

#[test]
fn duplicate_ssids_are_collapsed_into_one_display_row() {
    let rows = aggregate_wifi_networks(&[
        network("LabWiFi", "11", 61),
        network("DroneMesh", "1", 44),
        network("LabWiFi", "6", 78),
        network("LabWiFi", "1", 52),
    ]);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].ssid, "LabWiFi");
    assert_eq!(rows[0].signal, 78);
    assert_eq!(rows[0].instance_count, 3);
    assert_eq!(rows[0].channels, "1, 6, 11");
    assert_eq!(rows[1].ssid, "DroneMesh");
}

#[test]
fn connection_status_labels_do_not_embed_color_emoji_any_more() {
    let en = Lang::En;
    let zh = Lang::Zh;

    for key in [
        "conn_yes",
        "conn_yes_warn",
        "conn_wait",
        "conn_connecting",
        "conn_no",
    ] {
        assert!(!en.t(key).contains('🟢'));
        assert!(!en.t(key).contains('🟠'));
        assert!(!en.t(key).contains('🟡'));
        assert!(!en.t(key).contains('🔴'));
        assert!(!zh.t(key).contains('🟢'));
        assert!(!zh.t(key).contains('🟠'));
        assert!(!zh.t(key).contains('🟡'));
        assert!(!zh.t(key).contains('🔴'));
    }
}
