use std::time::Duration;

use tokio::sync::broadcast;
use tracing::{info, warn};

pub const REQUEST_FRAME_ACK_DELAY: Duration = Duration::from_millis(40);

pub fn schedule_transport_request_ack(
    tx: broadcast::Sender<Vec<u8>>,
    stream_id: u8,
    index: u8,
    likely_probe: bool,
) {
    if likely_probe {
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(REQUEST_FRAME_ACK_DELAY).await;
        match protocol::transport::encode_ack_frame(
            protocol::transport::FrameKind::AckRange,
            stream_id,
            index,
            crate::qos::TRANSPORT_FRAME_BUDGET,
        ) {
            Ok(frame) => {
                let _ = tx.send(frame);
                info!(
                    stream_id,
                    frame_index = index,
                    delay_ms = REQUEST_FRAME_ACK_DELAY.as_millis(),
                    "ble.request_frame.ack.sent"
                );
            }
            Err(err) => {
                warn!(
                    stream_id,
                    frame_index = index,
                    delay_ms = REQUEST_FRAME_ACK_DELAY.as_millis(),
                    error = %err,
                    "ble.request_frame.ack.failed"
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    #[tokio::test]
    async fn request_frame_ack_is_scheduled_after_write_callback_can_return() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<Vec<u8>>(4);

        super::schedule_transport_request_ack(tx, 7, 2, false);

        assert!(tokio::time::timeout(Duration::from_millis(10), rx.recv())
            .await
            .is_err());

        let raw = tokio::time::timeout(Duration::from_millis(250), rx.recv())
            .await
            .expect("ACK should be sent after the write callback has had time to return")
            .expect("broadcast channel should stay open");
        let frame = protocol::transport::decode_frame(&raw).unwrap();

        assert_eq!(frame.kind, protocol::transport::FrameKind::AckRange);
        assert_eq!(frame.stream_id, 7);
        assert_eq!(frame.index, 2);
    }

    #[tokio::test]
    async fn bootstrap_probe_does_not_schedule_request_ack() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<Vec<u8>>(4);
        let _keep_sender_alive = tx.clone();

        super::schedule_transport_request_ack(tx, 0xff, 0, true);

        assert!(tokio::time::timeout(Duration::from_millis(80), rx.recv())
            .await
            .is_err());
    }
}
