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
        })
        .await;

    assert!(!done);
    let mut retried_indexes = Vec::new();
    for _ in 0..chunk_count {
        retried_indexes.push(chunk_index(&rx.recv().await.unwrap()));
    }
    assert_eq!(retried_indexes, (1..=chunk_count).collect::<Vec<_>>());
}
