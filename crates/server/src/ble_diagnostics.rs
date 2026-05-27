use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub struct BleLinkDiagnostics {
    state: Arc<std::sync::Mutex<BleLinkDiagnosticsState>>,
    notify_seq: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
pub struct BleLinkSnapshot {
    pub notify_seq: u64,
    pub bytes: usize,
    pub frame_kind: Option<&'static str>,
    pub stream_id: Option<u8>,
    pub frame_index: Option<u8>,
    pub queued_for_ms: Option<u128>,
    pub notify_duration_ms: Option<u128>,
    pub ms_since_prev_notify: Option<u128>,
    pub last_ack_stream_id: Option<u8>,
    pub last_ack_index: Option<u8>,
    pub ms_since_last_ack: Option<u128>,
    pub last_write_kind: Option<&'static str>,
    pub last_write_stream_id: Option<u8>,
    pub last_write_index: Option<u8>,
    pub ms_since_last_write: Option<u128>,
}

#[derive(Default)]
struct BleLinkDiagnosticsState {
    last_notify: Option<NotifyRecord>,
    last_ack: Option<WriteRecord>,
    last_write: Option<WriteRecord>,
}

#[derive(Clone)]
struct NotifyRecord {
    seq: u64,
    bytes: usize,
    frame_kind: Option<&'static str>,
    stream_id: Option<u8>,
    frame_index: Option<u8>,
    queued_at: Instant,
    sent_at: Option<Instant>,
    duration: Option<Duration>,
}

#[derive(Clone)]
struct WriteRecord {
    frame_kind: Option<&'static str>,
    stream_id: Option<u8>,
    frame_index: Option<u8>,
    received_at: Instant,
}

impl BleLinkDiagnostics {
    pub fn record_write(&self, diagnostics: &WriteDiagnostics) {
        let mut state = self
            .state
            .lock()
            .expect("BLE link diagnostics lock poisoned");
        let record = WriteRecord {
            frame_kind: diagnostics.frame_kind,
            stream_id: diagnostics.stream_id,
            frame_index: diagnostics.frame_index,
            received_at: Instant::now(),
        };
        if diagnostics.frame_kind == Some("ack_range")
            || diagnostics.frame_kind == Some("ack_event")
        {
            state.last_ack = Some(record.clone());
        }
        state.last_write = Some(record);
    }

    pub fn next_notify(&self, raw: &[u8]) -> BleLinkSnapshot {
        let seq = self.notify_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let frame = protocol::transport::is_transport_frame(raw)
            .then(|| protocol::transport::decode_frame(raw).ok())
            .flatten();
        let record = NotifyRecord {
            seq,
            bytes: raw.len(),
            frame_kind: frame.as_ref().map(|frame| frame_kind_name(frame.kind)),
            stream_id: frame.as_ref().map(|frame| frame.stream_id),
            frame_index: frame.as_ref().map(|frame| frame.index),
            queued_at: Instant::now(),
            sent_at: None,
            duration: None,
        };
        let mut state = self
            .state
            .lock()
            .expect("BLE link diagnostics lock poisoned");
        let snapshot = snapshot_from_state(&state, &record, Instant::now());
        state.last_notify = Some(record);
        snapshot
    }

    pub fn mark_notify_finished(&self, seq: u64) -> Option<BleLinkSnapshot> {
        let mut state = self
            .state
            .lock()
            .expect("BLE link diagnostics lock poisoned");
        let now = Instant::now();
        if let Some(record) = state
            .last_notify
            .as_mut()
            .filter(|record| record.seq == seq)
        {
            record.sent_at = Some(now);
            record.duration = Some(now.duration_since(record.queued_at));
            let record = record.clone();
            return Some(snapshot_from_state(&state, &record, now));
        }
        None
    }

    pub fn snapshot(&self) -> Option<BleLinkSnapshot> {
        let state = self
            .state
            .lock()
            .expect("BLE link diagnostics lock poisoned");
        state
            .last_notify
            .as_ref()
            .map(|record| snapshot_from_state(&state, record, Instant::now()))
    }

    pub fn reset(&self) {
        let mut state = self
            .state
            .lock()
            .expect("BLE link diagnostics lock poisoned");
        *state = BleLinkDiagnosticsState::default();
    }
}

fn snapshot_from_state(
    state: &BleLinkDiagnosticsState,
    record: &NotifyRecord,
    now: Instant,
) -> BleLinkSnapshot {
    BleLinkSnapshot {
        notify_seq: record.seq,
        bytes: record.bytes,
        frame_kind: record.frame_kind,
        stream_id: record.stream_id,
        frame_index: record.frame_index,
        queued_for_ms: Some(now.duration_since(record.queued_at).as_millis()),
        notify_duration_ms: record.duration.map(|duration| duration.as_millis()),
        ms_since_prev_notify: state
            .last_notify
            .as_ref()
            .and_then(|previous| previous.sent_at.or(Some(previous.queued_at)))
            .map(|previous_at| now.duration_since(previous_at).as_millis()),
        last_ack_stream_id: state.last_ack.as_ref().and_then(|ack| ack.stream_id),
        last_ack_index: state.last_ack.as_ref().and_then(|ack| ack.frame_index),
        ms_since_last_ack: state
            .last_ack
            .as_ref()
            .map(|ack| now.duration_since(ack.received_at).as_millis()),
        last_write_kind: state.last_write.as_ref().and_then(|write| write.frame_kind),
        last_write_stream_id: state.last_write.as_ref().and_then(|write| write.stream_id),
        last_write_index: state
            .last_write
            .as_ref()
            .and_then(|write| write.frame_index),
        ms_since_last_write: state
            .last_write
            .as_ref()
            .map(|write| now.duration_since(write.received_at).as_millis()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteDiagnostics {
    pub payload_bytes: usize,
    pub frame_kind: Option<&'static str>,
    pub stream_id: Option<u8>,
    pub frame_index: Option<u8>,
    pub likely_probe: bool,
    pub preview_hex: String,
}

impl WriteDiagnostics {
    pub fn from_raw(raw: &[u8]) -> Self {
        let frame = protocol::transport::is_transport_frame(raw)
            .then(|| protocol::transport::decode_frame(raw).ok())
            .flatten();
        Self {
            payload_bytes: raw.len(),
            frame_kind: frame.as_ref().map(|frame| frame_kind_name(frame.kind)),
            stream_id: frame.as_ref().map(|frame| frame.stream_id),
            frame_index: frame.as_ref().map(|frame| frame.index),
            likely_probe: frame.as_ref().is_some_and(|frame| {
                frame.kind == protocol::transport::FrameKind::AckRange
                    && frame.stream_id == 0xff
                    && frame.index == 0
            }),
            preview_hex: preview_hex(raw, 24),
        }
    }
}

pub fn preview_hex(raw: &[u8], max_bytes: usize) -> String {
    let shown = raw
        .iter()
        .take(max_bytes)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if raw.len() > max_bytes {
        format!("{shown} ...")
    } else {
        shown
    }
}

pub fn frame_kind_name(kind: protocol::transport::FrameKind) -> &'static str {
    match kind {
        protocol::transport::FrameKind::RequestChunk => "request_chunk",
        protocol::transport::FrameKind::RequestFinal => "request_final",
        protocol::transport::FrameKind::ResponseChunk => "response_chunk",
        protocol::transport::FrameKind::ResponseFinal => "response_final",
        protocol::transport::FrameKind::AckRange => "ack_range",
        protocol::transport::FrameKind::AckEvent => "ack_event",
        protocol::transport::FrameKind::Reset => "reset",
        protocol::transport::FrameKind::Progress => "progress",
    }
}

#[cfg(test)]
mod tests {
    use super::{BleLinkDiagnostics, WriteDiagnostics};

    #[test]
    fn recognizes_miniprogram_bootstrap_probe() {
        let raw = protocol::transport::encode_ack_frame(
            protocol::transport::FrameKind::AckRange,
            0xff,
            0,
            20,
        )
        .unwrap();

        let diagnostics = WriteDiagnostics::from_raw(&raw);

        assert_eq!(diagnostics.payload_bytes, 4);
        assert_eq!(diagnostics.frame_kind, Some("ack_range"));
        assert_eq!(diagnostics.stream_id, Some(0xff));
        assert_eq!(diagnostics.frame_index, Some(0));
        assert_eq!(diagnostics.preview_hex, "59 25 ff 00");
        assert!(diagnostics.likely_probe);
    }

    #[test]
    fn classifies_request_payload_frame_without_marking_probe() {
        let request = protocol::CommandRequest::new(
            "req-transport",
            protocol::requests::CommandPayload::SystemStatus,
        );
        let payload = protocol::encode_request(&request).unwrap();
        let frame = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::RequestChunk,
            7,
            &payload,
            20,
        )
        .unwrap()
        .remove(0);

        let diagnostics = WriteDiagnostics::from_raw(&frame);

        assert_eq!(diagnostics.frame_kind, Some("request_chunk"));
        assert_eq!(diagnostics.stream_id, Some(7));
        assert_eq!(diagnostics.frame_index, Some(1));
        assert!(!diagnostics.likely_probe);
    }

    #[test]
    fn link_diagnostics_snapshot_tracks_last_ack_before_notify() {
        let link = BleLinkDiagnostics::default();
        let ack = protocol::transport::encode_ack_frame(
            protocol::transport::FrameKind::AckRange,
            9,
            4,
            20,
        )
        .unwrap();
        link.record_write(&WriteDiagnostics::from_raw(&ack));
        let response = protocol::transport::encode_control_frame(
            protocol::transport::FrameKind::Progress,
            3,
            1,
            20,
        )
        .unwrap();

        let snapshot = link.next_notify(&response);

        assert_eq!(snapshot.notify_seq, 1);
        assert_eq!(snapshot.last_ack_stream_id, Some(9));
        assert_eq!(snapshot.last_ack_index, Some(4));
        assert_eq!(snapshot.frame_kind, Some("progress"));
        assert_eq!(snapshot.stream_id, Some(3));
        assert_eq!(snapshot.frame_index, Some(1));
    }
}
