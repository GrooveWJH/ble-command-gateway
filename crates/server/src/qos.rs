use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};

const ACK_TIMEOUT: Duration = Duration::from_millis(750);
const EVENT_TTL: Duration = Duration::from_secs(60);
const MAX_RETRIES: u8 = 5;
const MAX_EVENTS: usize = 32;
pub const TRANSPORT_FRAME_BUDGET: usize = 20;
pub const TRANSPORT_WINDOW_SIZE: usize = 2;

#[derive(Clone)]
pub struct ReliableEventSender {
    tx: broadcast::Sender<Vec<u8>>,
    state: Arc<Mutex<ReliableState>>,
}

#[derive(Default)]
struct ReliableState {
    events: HashMap<EventKey, EventState>,
    next_response_stream_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EventKey {
    request_id: String,
    seq: u64,
    stream_id: Option<u8>,
}

struct EventState {
    command_name: String,
    chunks: Vec<Vec<u8>>,
    acked_chunks: HashSet<usize>,
    retry_counts: Vec<u8>,
    event_acked: bool,
    delivery: DeliveryMode,
    next_to_send: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryMode {
    LegacyJson,
    Transport {
        frame_budget: usize,
        window_size: usize,
    },
}

impl DeliveryMode {
    fn is_transport(self) -> bool {
        matches!(self, Self::Transport { .. })
    }

    fn transport_frame_budget(self) -> Option<usize> {
        match self {
            Self::LegacyJson => None,
            Self::Transport { frame_budget, .. } => Some(frame_budget),
        }
    }
}

impl ReliableEventSender {
    pub fn new(tx: broadcast::Sender<Vec<u8>>) -> Self {
        Self {
            tx,
            state: Arc::new(Mutex::new(ReliableState::default())),
        }
    }

    pub async fn send_event(&self, resp: protocol::CommandResponse, command_name: &str) {
        self.send_event_with_delivery(resp, command_name, DeliveryMode::LegacyJson)
            .await;
    }

    pub async fn send_event_with_delivery(
        &self,
        resp: protocol::CommandResponse,
        command_name: &str,
        delivery: DeliveryMode,
    ) {
        if resp.phase == protocol::ResponsePhase::Progress {
            if let Some(frame_budget) = delivery.transport_frame_budget() {
                if let Ok(raw) = protocol::transport::encode_control_frame(
                    protocol::transport::FrameKind::Progress,
                    0,
                    u8::try_from(resp.seq).unwrap_or(u8::MAX),
                    frame_budget,
                ) {
                    let _ = self.tx.send(raw);
                    info!(
                        request_id = %resp.id,
                        cmd = %command_name,
                        response_seq = resp.seq,
                        "ble.qos.progress.sent"
                    );
                }
                return;
            }
        }

        let (chunks, key, initial_send_count) = {
            let mut state = self.state.lock().await;
            let stream_id = delivery
                .is_transport()
                .then(|| state.allocate_response_stream_id());
            let chunks = encode_chunks(&resp, delivery, stream_id);
            let key = EventKey {
                request_id: resp.id.clone(),
                seq: resp.seq,
                stream_id,
            };
            let initial_send_count = initial_window_size(chunks.len(), delivery);
            if state.events.len() >= MAX_EVENTS {
                if let Some(first_key) = state.events.keys().next().cloned() {
                    state.events.remove(&first_key);
                    warn!(
                        request_id = %first_key.request_id,
                        response_seq = first_key.seq,
                        stream_id = ?first_key.stream_id,
                        "ble.qos.window_evicted"
                    );
                }
            }
            state.events.insert(
                key.clone(),
                EventState {
                    command_name: command_name.to_string(),
                    retry_counts: vec![0; chunks.len()],
                    chunks: chunks.clone(),
                    acked_chunks: HashSet::new(),
                    event_acked: false,
                    delivery,
                    next_to_send: initial_send_count + 1,
                },
            );
            (chunks, key, initial_send_count)
        };

        for (index, chunk) in chunks.iter().take(initial_send_count).enumerate() {
            self.send_chunk(&key, command_name, index + 1, chunk);
        }

        self.spawn_retry_loop(key);
    }

    pub async fn ack_chunk(&self, request_id: &str, response_seq: u64, chunk_index: usize) {
        let key = EventKey {
            request_id: request_id.to_string(),
            seq: response_seq,
            stream_id: None,
        };
        self.ack_chunk_by_key(key, chunk_index).await;
    }

    async fn ack_chunk_by_key(&self, key: EventKey, chunk_index: usize) {
        let mut state = self.state.lock().await;
        if let Some(event) = state.events.get_mut(&key) {
            event.acked_chunks.insert(chunk_index);
            info!(
                request_id = %key.request_id,
                response_seq = key.seq,
                stream_id = ?key.stream_id,
                chunk_index,
                "ble.qos.chunk.ack"
            );
            if let Some((command_name, next_index, chunk)) = next_transport_chunk(event) {
                drop(state);
                self.send_chunk(&key, &command_name, next_index, &chunk);
            }
        }
    }

    pub async fn ack_transport(
        &self,
        kind: protocol::transport::FrameKind,
        stream_id: u8,
        index: u8,
    ) {
        let key = {
            let state = self.state.lock().await;
            state
                .events
                .keys()
                .find(|key| key.stream_id == Some(stream_id))
                .cloned()
        };
        let Some(key) = key else {
            return;
        };

        match kind {
            protocol::transport::FrameKind::AckRange => {
                for chunk_index in 1..=usize::from(index) {
                    self.ack_chunk_by_key(key.clone(), chunk_index).await;
                }
            }
            protocol::transport::FrameKind::AckEvent => {
                self.ack_event_by_key(key).await;
            }
            _ => {}
        }
    }

    pub async fn ack_event(&self, request_id: &str, response_seq: u64) {
        let key = EventKey {
            request_id: request_id.to_string(),
            seq: response_seq,
            stream_id: None,
        };
        self.ack_event_by_key(key).await;
    }

    async fn ack_event_by_key(&self, key: EventKey) {
        let mut state = self.state.lock().await;
        if let Some(event) = state.events.get_mut(&key) {
            event.event_acked = true;
            info!(
                request_id = %key.request_id,
                response_seq = key.seq,
                stream_id = ?key.stream_id,
                "ble.qos.event.ack"
            );
        }
        state.events.remove(&key);
    }

    fn send_chunk(&self, key: &EventKey, command_name: &str, index: usize, chunk: &[u8]) {
        let _ = self.tx.send(chunk.to_vec());
        info!(
            request_id = %key.request_id,
            cmd = %command_name,
            response_seq = key.seq,
            chunk_index = index,
            chunk_bytes = chunk.len(),
            "ble.qos.chunk.sent"
        );
    }

    fn spawn_retry_loop(&self, key: EventKey) {
        let sender = self.clone();
        tokio::spawn(async move {
            let started = tokio::time::Instant::now();
            loop {
                tokio::time::sleep(ACK_TIMEOUT).await;
                if started.elapsed() > EVENT_TTL {
                    sender.drop_expired(&key).await;
                    return;
                }
                if sender.retry_missing_chunks(&key).await {
                    return;
                }
            }
        });
    }

    async fn retry_missing_chunks(&self, key: &EventKey) -> bool {
        let mut to_send = Vec::new();
        {
            let mut state = self.state.lock().await;
            let Some(event) = state.events.get_mut(key) else {
                return true;
            };
            if event.event_acked {
                return true;
            }
            let retry_indexes = retry_indexes(event);
            let retry_whole_event = matches!(event.delivery, DeliveryMode::LegacyJson)
                && !event.chunks.is_empty()
                && (1..=event.chunks.len()).all(|index| event.acked_chunks.contains(&index));
            for index in retry_indexes {
                if event.acked_chunks.contains(&index) && !retry_whole_event {
                    continue;
                }
                let retry_slot = &mut event.retry_counts[index - 1];
                if *retry_slot >= MAX_RETRIES {
                    warn!(
                        request_id = %key.request_id,
                        cmd = %event.command_name,
                        response_seq = key.seq,
                        stream_id = ?key.stream_id,
                        chunk_index = index,
                        "ble.qos.delivery_failed"
                    );
                    state.events.remove(key);
                    return true;
                }
                *retry_slot += 1;
                to_send.push((
                    event.command_name.clone(),
                    index,
                    event.chunks[index - 1].clone(),
                    *retry_slot,
                ));
            }
        }

        for (command_name, index, chunk, retry_count) in to_send {
            let _ = self.tx.send(chunk);
            info!(
                request_id = %key.request_id,
                cmd = %command_name,
                response_seq = key.seq,
                chunk_index = index,
                retry_count,
                "ble.qos.chunk.retry"
            );
        }
        false
    }

    async fn drop_expired(&self, key: &EventKey) {
        let mut state = self.state.lock().await;
        if state.events.remove(key).is_some() {
            warn!(
                request_id = %key.request_id,
                response_seq = key.seq,
                stream_id = ?key.stream_id,
                "ble.qos.delivery_failed"
            );
        }
    }
}

impl ReliableState {
    fn allocate_response_stream_id(&mut self) -> u8 {
        if self.next_response_stream_id == 0 {
            self.next_response_stream_id = 1;
        }
        let stream_id = self.next_response_stream_id;
        self.next_response_stream_id = self.next_response_stream_id.wrapping_add(1);
        if self.next_response_stream_id == 0 {
            self.next_response_stream_id = 1;
        }
        stream_id
    }
}

fn encode_chunks(
    resp: &protocol::CommandResponse,
    delivery: DeliveryMode,
    response_stream_id: Option<u8>,
) -> Vec<Vec<u8>> {
    match delivery {
        DeliveryMode::LegacyJson => protocol::chunking::chunk_response(resp.clone())
            .into_iter()
            .filter_map(|chunk| protocol::encode_response(&chunk).ok())
            .collect(),
        DeliveryMode::Transport {
            frame_budget,
            ..
        } => protocol::encode_response(resp)
            .ok()
            .and_then(|payload| {
                protocol::transport::encode_payload_frames(
                    protocol::transport::FrameKind::ResponseChunk,
                    response_stream_id?,
                    &payload,
                    frame_budget,
                )
                .ok()
            })
            .unwrap_or_default(),
    }
}

fn initial_window_size(chunk_count: usize, delivery: DeliveryMode) -> usize {
    match delivery {
        DeliveryMode::LegacyJson => chunk_count,
        DeliveryMode::Transport { window_size, .. } => chunk_count.min(window_size.max(1)),
    }
}

fn next_transport_chunk(event: &mut EventState) -> Option<(String, usize, Vec<u8>)> {
    let DeliveryMode::Transport { window_size, .. } = event.delivery else {
        return None;
    };
    let in_flight = (1..event.next_to_send)
        .filter(|index| !event.acked_chunks.contains(index))
        .count();
    if in_flight >= window_size || event.next_to_send > event.chunks.len() {
        return None;
    }
    let index = event.next_to_send;
    event.next_to_send += 1;
    Some((
        event.command_name.clone(),
        index,
        event.chunks[index - 1].clone(),
    ))
}

fn retry_indexes(event: &EventState) -> Vec<usize> {
    match event.delivery {
        DeliveryMode::LegacyJson => (1..=event.chunks.len()).collect(),
        DeliveryMode::Transport { .. } => (1..event.next_to_send)
            .filter(|index| !event.acked_chunks.contains(index))
            .collect(),
    }
}

#[cfg(test)]
mod tests;
