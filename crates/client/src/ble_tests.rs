use crate::ble::{progress_event, sort_scan_candidates, ScanCandidateInfo, ScanProgressEvent};

#[test]
fn sort_scan_candidates_orders_devices_by_signal_then_name() {
    let mut candidates = vec![
        ScanCandidateInfo {
            name: "Yundrone_UAV-15-19-A7F2".to_string(),
            rssi: Some(-60),
        },
        ScanCandidateInfo {
            name: "Other_Device".to_string(),
            rssi: Some(-20),
        },
        ScanCandidateInfo {
            name: "Yundrone_UAV-15-20-B110".to_string(),
            rssi: Some(-45),
        },
    ];

    sort_scan_candidates(&mut candidates);

    assert_eq!(candidates[0].name, "Other_Device");
    assert_eq!(candidates[1].name, "Yundrone_UAV-15-20-B110");
    assert_eq!(candidates[2].name, "Yundrone_UAV-15-19-A7F2");
}

#[test]
fn sort_scan_candidates_puts_unknown_rssi_last() {
    let mut candidates = vec![
        ScanCandidateInfo {
            name: "Yundrone_UAV-15-21-UNK".to_string(),
            rssi: None,
        },
        ScanCandidateInfo {
            name: "Yundrone_UAV-15-20-B110".to_string(),
            rssi: Some(-45),
        },
    ];

    sort_scan_candidates(&mut candidates);

    assert_eq!(candidates[0].name, "Yundrone_UAV-15-20-B110");
    assert_eq!(candidates[1].name, "Yundrone_UAV-15-21-UNK");
}

#[test]
fn progress_event_captures_name_signal_and_prefix_match() {
    assert_eq!(
        progress_event("Yundrone_UAV-15-19-A7".to_string(), Some(-41), true),
        ScanProgressEvent {
            device_name: "Yundrone_UAV-15-19-A7".to_string(),
            rssi: Some(-41),
            matches_prefix: true,
        }
    );
}
