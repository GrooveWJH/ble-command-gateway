use super::*;

fn chunk_index(raw: &[u8]) -> usize {
    let response = protocol::parse_response(raw).unwrap();
    response
        .data
        .as_ref()
        .and_then(|data| data.get("chunk"))
        .and_then(|chunk| chunk.get("index"))
        .and_then(|index| index.as_u64())
        .unwrap() as usize
}

#[test]
fn reliable_sender_constants_match_qos_policy() {
    assert_eq!(ACK_TIMEOUT, std::time::Duration::from_millis(750));
    assert_eq!(MAX_RETRIES, 5);
    assert_eq!(MAX_EVENTS, 32);
    assert_eq!(TRANSPORT_WINDOW_SIZE, 2);
}

#[tokio::test]
async fn acknowledged_chunk_is_not_retried_while_other_chunks_are_missing() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let sender = ReliableEventSender::new(tx);
    let response = protocol::CommandResponse::ok("req-qos", "x".repeat(500), None);
    let chunk_count = protocol::chunking::chunk_response(response.clone()).len();
    assert!(chunk_count > 1);

    sender.send_event(response, "wifi.scan").await;
    for _ in 0..chunk_count {
        let _ = rx.recv().await.unwrap();
    }
    sender.ack_chunk("req-qos", 1, 1).await;
    let done = sender
        .retry_missing_chunks(&EventKey {
            request_id: "req-qos".to_string(),
            seq: 1,
            stream_id: None,
        })
        .await;

    assert!(!done);
    let mut retried_indexes = Vec::new();
    for _ in 0..chunk_count - 1 {
        retried_indexes.push(chunk_index(&rx.recv().await.unwrap()));
    }
    assert!(!retried_indexes.contains(&1));
}

#[tokio::test]
async fn event_ack_removes_cached_event() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let sender = ReliableEventSender::new(tx);
    let response = protocol::CommandResponse::ok("req-event", "ok", None);

    sender.send_event(response, "system.status").await;
    let _ = rx.recv().await.unwrap();
    sender.ack_event("req-event", 1).await;
    let done = sender
        .retry_missing_chunks(&EventKey {
            request_id: "req-event".to_string(),
            seq: 1,
            stream_id: None,
        })
        .await;

    assert!(done);
}

#[tokio::test]
async fn all_chunk_acks_without_event_ack_retry_the_whole_event() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let sender = ReliableEventSender::new(tx);
    let response = protocol::CommandResponse::ok("req-event-ack", "x".repeat(500), None);
    let chunk_count = protocol::chunking::chunk_response(response.clone()).len();

    sender.send_event(response, "wifi.scan").await;
    for index in 1..=chunk_count {
        let _ = rx.recv().await.unwrap();
        sender.ack_chunk("req-event-ack", 1, index).await;
    }
    let done = sender
        .retry_missing_chunks(&EventKey {
            request_id: "req-event-ack".to_string(),
            seq: 1,
            stream_id: None,
        })
        .await;

    assert!(!done);
    let mut retried_indexes = Vec::new();
    for _ in 0..chunk_count {
        retried_indexes.push(chunk_index(&rx.recv().await.unwrap()));
    }
    assert_eq!(retried_indexes, (1..=chunk_count).collect::<Vec<_>>());
}

#[tokio::test]
async fn transport_delivery_sends_window_one_and_uses_compact_acks() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let sender = ReliableEventSender::new(tx);
    let response = protocol::CommandResponse::ok("req-transport", "x".repeat(500), None);

    sender
        .send_event_with_delivery(
            response,
            "system.status",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        )
        .await;

    let first = rx.recv().await.unwrap();
    let frame = protocol::transport::decode_frame(&first).unwrap();
    assert_eq!(frame.kind, protocol::transport::FrameKind::ResponseChunk);
    assert_eq!(frame.index, 1);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), rx.recv())
            .await
            .is_err()
    );

    sender
        .ack_transport(protocol::transport::FrameKind::AckRange, frame.stream_id, 1)
        .await;
    let second = rx.recv().await.unwrap();
    let second_frame = protocol::transport::decode_frame(&second).unwrap();
    assert_eq!(second_frame.stream_id, frame.stream_id);
    assert_eq!(second_frame.index, 2);

    sender
        .ack_transport(protocol::transport::FrameKind::AckEvent, frame.stream_id, 0)
        .await;
    let done = sender
        .retry_missing_chunks(&EventKey {
            request_id: "req-transport".to_string(),
            seq: 1,
            stream_id: Some(frame.stream_id),
        })
        .await;
    assert!(done);
}

#[tokio::test]
async fn transport_retry_respects_response_window() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let sender = ReliableEventSender::new(tx);
    let response = protocol::CommandResponse::ok("req-window", "x".repeat(500), None);

    sender
        .send_event_with_delivery(
            response,
            "system.status",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        )
        .await;
    let first = rx.recv().await.unwrap();
    let first_frame = protocol::transport::decode_frame(&first).unwrap();

    let done = sender
        .retry_missing_chunks(&EventKey {
            request_id: "req-window".to_string(),
            seq: 1,
            stream_id: Some(first_frame.stream_id),
        })
        .await;

    assert!(!done);
    let retry = rx.recv().await.unwrap();
    let retry_frame = protocol::transport::decode_frame(&retry).unwrap();
    assert_eq!(retry_frame.index, first_frame.index);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), rx.recv())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn transport_window_two_sends_two_initial_frames() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let sender = ReliableEventSender::new(tx);
    let response = protocol::CommandResponse::ok("req-window-two", "x".repeat(500), None);

    sender
        .send_event_with_delivery(
            response,
            "system.status",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 2,
            },
        )
        .await;

    let first = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();
    let second = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();

    assert_eq!(first.index, 1);
    assert_eq!(second.index, 2);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), rx.recv())
            .await
            .is_err()
    );

    sender
        .ack_transport(protocol::transport::FrameKind::AckRange, first.stream_id, 1)
        .await;
    let third = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();
    assert_eq!(third.index, 3);
}

#[tokio::test]
async fn transport_ack_range_two_advances_window_by_two_frames() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let sender = ReliableEventSender::new(tx);
    let response = protocol::CommandResponse::ok("req-range-window", "x".repeat(500), None);

    sender
        .send_event_with_delivery(
            response,
            "system.status",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 2,
            },
        )
        .await;

    let first = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();
    let second = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();
    assert_eq!((first.index, second.index), (1, 2));

    sender
        .ack_transport(protocol::transport::FrameKind::AckRange, first.stream_id, 2)
        .await;

    let third = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();
    let fourth = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();
    assert_eq!((third.index, fourth.index), (3, 4));
}

#[tokio::test]
async fn duplicate_transport_event_does_not_overwrite_in_flight_event() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(32);
    let sender = ReliableEventSender::new(tx);
    let first_response = protocol::CommandResponse::ok("req-duplicate", "first".repeat(80), None);
    let second_response = protocol::CommandResponse::ok("req-duplicate", "second".repeat(80), None);

    sender
        .send_event_with_delivery(
            first_response,
            "system.status",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        )
        .await;
    let first_frame = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();

    sender
        .send_event_with_delivery(
            second_response,
            "system.status",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        )
        .await;
    let second_frame = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();

    assert_ne!(first_frame.stream_id, second_frame.stream_id);

    sender
        .ack_transport(
            protocol::transport::FrameKind::AckRange,
            first_frame.stream_id,
            first_frame.index,
        )
        .await;
    let first_next = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();
    assert_eq!(first_next.stream_id, first_frame.stream_id);
    assert_eq!(first_next.index, 2);
}

#[tokio::test]
async fn transport_events_allocate_distinct_response_streams_for_same_request() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(32);
    let sender = ReliableEventSender::new(tx);
    let accepted = protocol::CommandResponse::accepted(
        "req-long".to_string(),
        "wifi.scan".to_string(),
        "accepted",
        None,
    );
    let mut result = protocol::CommandResponse::ok("req-long", "wifi scan complete", None);
    result.seq = 2;

    sender
        .send_event_with_delivery(
            accepted,
            "wifi.scan",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        )
        .await;
    let accepted_frame = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();

    sender
        .send_event_with_delivery(
            result,
            "wifi.scan",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        )
        .await;
    let result_frame = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();

    assert_ne!(accepted_frame.stream_id, result_frame.stream_id);
    assert_eq!(accepted_frame.index, 1);
    assert_eq!(result_frame.index, 1);
}

#[tokio::test]
async fn transport_acks_are_scoped_to_response_stream_id() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(32);
    let sender = ReliableEventSender::new(tx);
    let first_response = protocol::CommandResponse::ok("req-scope", "first".repeat(80), None);
    let mut second_response = protocol::CommandResponse::ok("req-scope", "second".repeat(80), None);
    second_response.seq = 2;

    sender
        .send_event_with_delivery(
            first_response,
            "wifi.scan",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        )
        .await;
    let first_frame = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();

    sender
        .send_event_with_delivery(
            second_response,
            "wifi.scan",
            DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        )
        .await;
    let second_frame = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();

    sender
        .ack_transport(
            protocol::transport::FrameKind::AckRange,
            first_frame.stream_id,
            1,
        )
        .await;
    let first_next = protocol::transport::decode_frame(&rx.recv().await.unwrap()).unwrap();
    assert_eq!(first_next.stream_id, first_frame.stream_id);

    sender
        .ack_transport(
            protocol::transport::FrameKind::AckEvent,
            first_frame.stream_id,
            0,
        )
        .await;
    let first_done = sender
        .retry_missing_chunks(&EventKey {
            request_id: "req-scope".to_string(),
            seq: 1,
            stream_id: Some(first_frame.stream_id),
        })
        .await;
    let second_done = sender
        .retry_missing_chunks(&EventKey {
            request_id: "req-scope".to_string(),
            seq: 2,
            stream_id: Some(second_frame.stream_id),
        })
        .await;

    assert!(first_done);
    assert!(!second_done);
}
