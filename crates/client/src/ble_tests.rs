use crate::ble::{
    progress_event, raw_identity_scan_filter, sort_scan_candidates, ScanCandidateInfo,
    ScanProgressEvent,
};

#[test]
fn sort_scan_candidates_orders_devices_by_signal_then_name() {
    let mut candidates = vec![
        ScanCandidateInfo {
            name: "yundrone-00d8ab".to_string(),
            rssi: Some(-60),
        },
        ScanCandidateInfo {
            name: "Other_Device".to_string(),
            rssi: Some(-20),
        },
        ScanCandidateInfo {
            name: "yundrone-00b110".to_string(),
            rssi: Some(-45),
        },
    ];

    sort_scan_candidates(&mut candidates);

    assert_eq!(candidates[0].name, "Other_Device");
    assert_eq!(candidates[1].name, "yundrone-00b110");
    assert_eq!(candidates[2].name, "yundrone-00d8ab");
}

#[test]
fn sort_scan_candidates_puts_unknown_rssi_last() {
    let mut candidates = vec![
        ScanCandidateInfo {
            name: "yundrone-000000".to_string(),
            rssi: None,
        },
        ScanCandidateInfo {
            name: "yundrone-00b110".to_string(),
            rssi: Some(-45),
        },
    ];

    sort_scan_candidates(&mut candidates);

    assert_eq!(candidates[0].name, "yundrone-00b110");
    assert_eq!(candidates[1].name, "yundrone-000000");
}

#[test]
fn progress_event_captures_name_signal_and_prefix_match() {
    assert_eq!(
        progress_event("yundrone-00a700".to_string(), Some(-41), true),
        ScanProgressEvent {
            device_name: "yundrone-00a700".to_string(),
            rssi: Some(-41),
            matches_prefix: true,
        }
    );
}

#[test]
fn raw_identity_scan_filter_does_not_hide_non_matching_devices() {
    let filter = raw_identity_scan_filter();

    assert!(
        filter.services.is_empty(),
        "scan progress should see all named BLE identities; candidates are filtered later"
    );
}
