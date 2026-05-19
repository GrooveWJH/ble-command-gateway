use super::*;

fn assert_response_data_round_trip<T>(value: T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let map = responses::to_map(&value).unwrap();
    let decoded: T = responses::from_map(&map).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn every_typed_request_round_trips_through_wire_schema() {
    let requests = vec![
        CommandRequest::new(
            "req-capabilities",
            requests::CommandPayload::SystemCapabilities,
        ),
        CommandRequest::new("req-heartbeat", requests::CommandPayload::LinkHeartbeat),
        CommandRequest::new("req-status", requests::CommandPayload::SystemStatus),
        CommandRequest::new(
            "req-profiles-list",
            requests::CommandPayload::WifiProfilesList,
        ),
        CommandRequest::new(
            "req-scan",
            requests::CommandPayload::WifiScan {
                ifname: Some("wlan1".to_string()),
            },
        ),
        CommandRequest::new(
            "req-provision",
            requests::CommandPayload::WifiProvision {
                ssid: "LabWiFi".to_string(),
                pwd: Some("secret".to_string()),
            },
        ),
        CommandRequest::new(
            "req-profiles-delete",
            requests::CommandPayload::WifiProfilesDelete {
                uuids: vec!["profile-uuid".to_string()],
                force: false,
            },
        ),
    ];

    for request in requests {
        let encoded = encode_request(&request).unwrap();
        let decoded = parse_request(&encoded).unwrap();
        assert_eq!(decoded, request);
    }
}

#[test]
fn parse_request_rejects_missing_id() {
    let err = parse_request(br#"{"cmd":"system.status","args":{}}"#).unwrap_err();

    match err {
        ProtocolError::BadJson(message) => {
            assert!(message.contains("missing field"));
            assert!(message.contains("id"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn parse_request_rejects_bad_command_args() {
    let err =
        parse_request(br#"{"id":"req-1","cmd":"system.status","args":{"bad":true}}"#).unwrap_err();

    match err {
        ProtocolError::BadRequest(message) => {
            assert!(message.contains("does not accept args"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn parse_request_rejects_v1_command_names() {
    let err = parse_request(br#"{"id":"req-old","cmd":"ping","args":{}}"#).unwrap_err();

    match err {
        ProtocolError::BadRequest(message) => {
            assert!(message.contains("unknown command: ping"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn parse_request_rejects_v1_protocol_version() {
    let err = parse_request(
        br#"{"id":"req-old-version","cmd":"link.heartbeat","args":{},"v":"YundroneBT-V1.0.0"}"#,
    )
    .unwrap_err();

    match err {
        ProtocolError::BadRequest(message) => {
            assert!(message.contains("unsupported protocol version"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn parse_error_response_preserves_request_identity_for_debuggers() {
    let raw = br#"{"id":"req-old","cmd":"ping","args":{},"v":"YundroneBT-V2.0.0"}"#;
    let err = parse_request(raw).unwrap_err();
    let response = parse_error_response(raw, &err).expect("id/cmd should be recoverable");

    assert_eq!(response.id, "req-old");
    assert_eq!(response.cmd.as_deref(), Some("ping"));
    assert!(!response.ok);
    assert_eq!(response.code, codes::CODE_UNKNOWN_COMMAND);
    assert!(response.final_flag);
}

#[test]
fn parse_request_decodes_typed_wifi_scan() {
    let decoded =
        parse_request(br#"{"id":"req-2","cmd":"wifi.scan","args":{"ifname":"wlan0"}}"#).unwrap();

    assert_eq!(
        decoded.payload,
        requests::CommandPayload::WifiScan {
            ifname: Some("wlan0".to_string())
        }
    );
}

#[test]
fn every_typed_response_data_round_trips_through_json_maps() {
    assert_response_data_round_trip(responses::HeartbeatResponseData { alive: true });
    assert_response_data_round_trip(responses::StatusResponseData {
        device_name: "yundrone-15-19-a7f2".to_string(),
        hostname: "edge-gateway".to_string(),
        system: "Ubuntu".to_string(),
        user: "demo-user".to_string(),
        network: Some("LabWiFi".to_string()),
        ip: Some("192.0.2.2".to_string()),
        interfaces: vec![
            responses::StatusInterfaceIpv4 {
                ifname: "wlan0".to_string(),
                kind: responses::StatusInterfaceKind::Wifi,
                ipv4: "192.0.2.2".to_string(),
            },
            responses::StatusInterfaceIpv4 {
                ifname: "eth0".to_string(),
                kind: responses::StatusInterfaceKind::Ethernet,
                ipv4: "198.51.100.8".to_string(),
            },
        ],
    });
    assert_response_data_round_trip(responses::CapabilitiesResponseData {
        protocol_version: PROTOCOL_VERSION.to_string(),
        commands: vec!["system.status".to_string()],
        features: vec!["response_events".to_string()],
        payload_limit: config::MAX_BLE_PAYLOAD_BYTES,
    });
    assert_response_data_round_trip(responses::WifiScanResponseData {
        ifname: Some("wlan0".to_string()),
        count: 2,
        networks: vec![
            responses::WifiNetwork {
                ssid: "LabWiFi".to_string(),
                channel: "6".to_string(),
                signal: 78,
            },
            responses::WifiNetwork {
                ssid: "DroneDebug".to_string(),
                channel: "11".to_string(),
                signal: 61,
            },
        ],
    });
    assert_response_data_round_trip(responses::ProvisionResponseData {
        status: responses::ProvisionState::Connected,
        ssid: "LabWiFi".to_string(),
        ip: Some("192.0.2.2".to_string()),
    });
    assert_response_data_round_trip(responses::WifiProfilesResponseData {
        profiles: vec![responses::WifiProfile {
            uuid: "profile-uuid".to_string(),
            name: "LabWiFi".to_string(),
            ssid: "LabWiFi".to_string(),
            active: true,
            device: Some("wlan0".to_string()),
            autoconnect: true,
        }],
    });
    assert_response_data_round_trip(responses::WifiProfilesDeleteResponseData {
        deleted: vec![responses::WifiProfileDeleteItem {
            uuid: "deleted-uuid".to_string(),
            name: "OldWiFi".to_string(),
            ssid: "OldWiFi".to_string(),
        }],
        skipped: vec![responses::WifiProfileSkippedItem {
            uuid: "active-uuid".to_string(),
            name: "LabWiFi".to_string(),
            ssid: "LabWiFi".to_string(),
            reason: "active_profile".to_string(),
        }],
        failed: vec![responses::WifiProfileFailedItem {
            uuid: "missing-uuid".to_string(),
            name: "Missing".to_string(),
            ssid: "Missing".to_string(),
            error: "not found".to_string(),
        }],
    });
}

#[test]
fn command_response_events_preserve_phase_metadata() {
    let response = CommandResponse::progress(
        "req-progress",
        commands::CMD_WIFI_SCAN,
        2,
        "still scanning",
        None,
    );
    let encoded = encode_response(&response).unwrap();
    let decoded = parse_response(&encoded).unwrap();

    assert_eq!(decoded.cmd.as_deref(), Some(commands::CMD_WIFI_SCAN));
    assert_eq!(decoded.phase, ResponsePhase::Progress);
    assert_eq!(decoded.seq, 2);
    assert!(!decoded.final_flag);
    assert_eq!(decoded.v, PROTOCOL_VERSION);
}

#[test]
fn response_phase_as_str_matches_wire_names() {
    assert_eq!(ResponsePhase::Accepted.as_str(), "accepted");
    assert_eq!(ResponsePhase::Progress.as_str(), "progress");
    assert_eq!(ResponsePhase::Result.as_str(), "result");
}

#[test]
fn large_status_data_response_chunks_and_round_trips() {
    let response_data = responses::StatusResponseData {
        device_name: "yundrone-15-19-a7f2".to_string(),
        hostname: "edge-linux-deployment-target".repeat(4),
        system: "Linux 6.1.0-jetson aarch64".repeat(4),
        user: "yundrone".to_string(),
        network: Some("FieldOpsMesh".repeat(4)),
        ip: Some("192.0.2.2".to_string()),
        interfaces: vec![
            responses::StatusInterfaceIpv4 {
                ifname: "wlan0".to_string(),
                kind: responses::StatusInterfaceKind::Wifi,
                ipv4: "192.0.2.2".to_string(),
            },
            responses::StatusInterfaceIpv4 {
                ifname: "wlan1".to_string(),
                kind: responses::StatusInterfaceKind::Wifi,
                ipv4: "203.0.113.22".to_string(),
            },
            responses::StatusInterfaceIpv4 {
                ifname: "eth0".to_string(),
                kind: responses::StatusInterfaceKind::Ethernet,
                ipv4: "198.51.100.9".to_string(),
            },
        ],
    };
    let response = CommandResponse::ok(
        "req-large-status",
        "status collected",
        Some(responses::to_map(&response_data).unwrap()),
    );

    let chunks = chunking::chunk_response(response.clone());

    assert!(chunks.len() > 1);

    let mut assembler = chunking::ChunkAssembler::new();
    let mut assembled = None;
    for chunk in chunks {
        let encoded = encode_response(&chunk).unwrap();
        let decoded = parse_response(&encoded).unwrap();
        assembled = assembler.add_chunk(decoded).unwrap();
    }

    let assembled = assembled.expect("status response should reassemble");
    let decoded_data: responses::StatusResponseData = assembled.decode_data().unwrap();
    assert_eq!(assembled, response);
    assert_eq!(decoded_data, response_data);
}

#[test]
fn response_round_trip_preserves_schema() {
    let data = responses::to_map(&responses::WifiScanResponseData {
        ifname: None,
        count: 0,
        networks: vec![],
    })
    .unwrap();
    let response = CommandResponse::ok("req-2", "wifi scan complete", Some(data.clone()));

    let encoded = encode_response(&response).unwrap();
    let decoded = parse_response(&encoded).unwrap();

    assert_eq!(decoded.id, "req-2");
    assert!(decoded.ok);
    assert_eq!(decoded.code, codes::CODE_OK);
    assert_eq!(decoded.text, "wifi scan complete");
    assert_eq!(decoded.data, Some(data));
    assert_eq!(decoded.v, PROTOCOL_VERSION);
}

#[test]
fn chunked_response_round_trip_preserves_typed_data() {
    let response_data = responses::WifiScanResponseData {
        ifname: Some("wlan0".to_string()),
        count: 1,
        networks: vec![responses::WifiNetwork {
            ssid: "LabWiFi".to_string(),
            channel: "6".to_string(),
            signal: 78,
        }],
    };
    let response = CommandResponse::ok(
        "req-3",
        "wifi scan complete ".repeat(40),
        Some(responses::to_map(&response_data).unwrap()),
    );
    let chunks = chunking::chunk_response(response.clone());
    let mut assembler = chunking::ChunkAssembler::new();
    let mut assembled = None;

    assert!(chunks.len() > 1);

    for chunk in chunks {
        let encoded = encode_response(&chunk).unwrap();
        let decoded = parse_response(&encoded).unwrap();
        assembled = assembler.add_chunk(decoded).unwrap();
    }

    let assembled = assembled.expect("chunked response should reassemble");
    let decoded_data: responses::WifiScanResponseData = assembled.decode_data().unwrap();

    assert_eq!(assembled, response);
    assert_eq!(decoded_data, response_data);
}

#[test]
fn large_data_response_chunks_even_when_text_is_short() {
    let response_data = responses::WifiScanResponseData {
        ifname: Some("wlan0".to_string()),
        count: 20,
        networks: (0..20)
            .map(|index| responses::WifiNetwork {
                ssid: format!("LabWiFi-{index:02}-EXTREMELY-LONG-NAME"),
                channel: ((index % 11) + 1).to_string(),
                signal: 90 - index,
            })
            .collect(),
    };
    let response = CommandResponse::ok(
        "req-large-data",
        "wifi scan complete",
        Some(responses::to_map(&response_data).unwrap()),
    );

    let chunks = chunking::chunk_response(response.clone());

    assert!(
        chunks.len() > 1,
        "large data response should be chunked even when text is short"
    );
    for chunk in &chunks {
        let encoded = encode_response(chunk).unwrap();
        assert!(
            encoded.len() <= config::MAX_BLE_PAYLOAD_BYTES,
            "chunk payload exceeded BLE limit: {} > {}",
            encoded.len(),
            config::MAX_BLE_PAYLOAD_BYTES
        );
    }

    let mut assembler = chunking::ChunkAssembler::new();
    let mut assembled = None;
    for chunk in chunks {
        let encoded = encode_response(&chunk).unwrap();
        let decoded = parse_response(&encoded).unwrap();
        assembled = assembler.add_chunk(decoded).unwrap();
    }

    let assembled = assembled.expect("chunked large data response should reassemble");
    let decoded_data: responses::WifiScanResponseData = assembled.decode_data().unwrap();
    assert_eq!(assembled, response);
    assert_eq!(decoded_data, response_data);
}
