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
        CommandRequest::new("req-help", requests::CommandPayload::Help),
        CommandRequest::new("req-ping", requests::CommandPayload::Ping),
        CommandRequest::new("req-status", requests::CommandPayload::Status),
        CommandRequest::new("req-whoami", requests::CommandPayload::SysWhoAmI),
        CommandRequest::new(
            "req-ifconfig",
            requests::CommandPayload::NetIfconfig {
                ifname: Some("wlan0".to_string()),
            },
        ),
        CommandRequest::new(
            "req-scan",
            requests::CommandPayload::WifiScan {
                ifname: Some("wlan1".to_string()),
            },
        ),
        CommandRequest::new(
            "req-provision",
            requests::CommandPayload::Provision {
                ssid: "LabWiFi".to_string(),
                pwd: Some("secret".to_string()),
            },
        ),
        CommandRequest::new("req-shutdown", requests::CommandPayload::Shutdown),
    ];

    for request in requests {
        let encoded = encode_request(&request).unwrap();
        let decoded = parse_request(&encoded).unwrap();
        assert_eq!(decoded, request);
    }
}

#[test]
fn parse_request_rejects_missing_id() {
    let err = parse_request(br#"{"cmd":"status","args":{}}"#).unwrap_err();

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
    let err = parse_request(br#"{"id":"req-1","cmd":"status","args":{"bad":true}}"#).unwrap_err();

    match err {
        ProtocolError::BadRequest(message) => {
            assert!(message.contains("does not accept args"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
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
    assert_response_data_round_trip(responses::HelpResponseData {
        commands: vec!["status".to_string(), "wifi.scan".to_string()],
    });
    assert_response_data_round_trip(responses::PingResponseData { pong: true });
    assert_response_data_round_trip(responses::StatusResponseData {
        hostname: "orin".to_string(),
        system: "Ubuntu".to_string(),
        user: "orangepi".to_string(),
        network: Some("LabWiFi".to_string()),
        ip: Some("192.168.10.2".to_string()),
        interfaces: vec![
            responses::StatusInterfaceIpv4 {
                ifname: "wlan0".to_string(),
                kind: responses::StatusInterfaceKind::Wifi,
                ipv4: "192.168.10.2".to_string(),
            },
            responses::StatusInterfaceIpv4 {
                ifname: "eth0".to_string(),
                kind: responses::StatusInterfaceKind::Ethernet,
                ipv4: "10.0.0.8".to_string(),
            },
        ],
    });
    assert_response_data_round_trip(responses::WhoAmIResponseData {
        user: "root".to_string(),
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
        ip: Some("192.168.10.2".to_string()),
    });
}

#[test]
fn large_status_data_response_chunks_and_round_trips() {
    let response_data = responses::StatusResponseData {
        hostname: "orin-nx-deployment-target".repeat(4),
        system: "Linux 6.1.0-jetson aarch64".repeat(4),
        user: "yundrone".to_string(),
        network: Some("FieldOpsMesh".repeat(4)),
        ip: Some("192.168.10.2".to_string()),
        interfaces: vec![
            responses::StatusInterfaceIpv4 {
                ifname: "wlan0".to_string(),
                kind: responses::StatusInterfaceKind::Wifi,
                ipv4: "192.168.10.2".to_string(),
            },
            responses::StatusInterfaceIpv4 {
                ifname: "wlan1".to_string(),
                kind: responses::StatusInterfaceKind::Wifi,
                ipv4: "172.16.0.22".to_string(),
            },
            responses::StatusInterfaceIpv4 {
                ifname: "eth0".to_string(),
                kind: responses::StatusInterfaceKind::Ethernet,
                ipv4: "10.24.6.9".to_string(),
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
