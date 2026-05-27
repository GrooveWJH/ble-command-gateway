use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};

const ACK_TIMEOUT: Duration = Duration::from_millis(750);
const EVENT_TTL: Duration = Duration::from_secs(60);
const MAX_RETRIES: u8 = 5;
const MAX_EVENTS: usize = 32;
pub const TRANSPORT_FRAME_BUDGET: usize = protocol::transport::MAX_FRAME_BUDGET;
pub const TRANSPORT_WINDOW_SIZE: usize = 1;
pub const DEFAULT_POST_ACK_NOTIFY_DELAY_MS: u64 = 180;

const POST_ACK_NOTIFY_DELAY_ENV: &str = "YUNDRONE_BLE_POST_ACK_NOTIFY_DELAY_MS";

#[derive(Clone)]
pub struct ReliableEventSender {
    tx: broadcast::Sender<Vec<u8>>,
    state: Arc<Mutex<ReliableState>>,
    post_ack_notify_delay: Duration,
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
    last_sent_at: Vec<Option<Instant>>,
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

    pub fn transport_frame_budget(self) -> Option<usize> {
        match self {
            Self::LegacyJson => None,
            Self::Transport { frame_budget, .. } => Some(frame_budget),
        }
    }

    pub fn transport_window_size(self) -> Option<usize> {
        match self {
            Self::LegacyJson => None,
            Self::Transport { window_size, .. } => Some(window_size),
        }
    }
}

impl ReliableEventSender {
    pub fn new(tx: broadcast::Sender<Vec<u8>>) -> Self {
        Self::with_post_ack_notify_delay(tx, configured_post_ack_notify_delay())
    }

    pub fn with_post_ack_notify_delay(
        tx: broadcast::Sender<Vec<u8>>,
        post_ack_notify_delay: Duration,
    ) -> Self {
        Self {
            tx,
            state: Arc::new(Mutex::new(ReliableState::default())),
            post_ack_notify_delay,
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
            let mut last_sent_at = vec![None; chunks.len()];
            let now = Instant::now();
            for slot in last_sent_at.iter_mut().take(initial_send_count) {
                *slot = Some(now);
            }
            state.events.insert(
                key.clone(),
                EventState {
                    command_name: command_name.to_string(),
                    retry_counts: vec![0; chunks.len()],
                    last_sent_at,
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
            self.send_chunk(&key, command_name, index + 1, chunk, SendReason::Initial);
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
            if !event.acked_chunks.insert(chunk_index) {
                return;
            }
            info!(
                request_id = %key.request_id,
                response_seq = key.seq,
                stream_id = ?key.stream_id,
                chunk_index,
                "ble.qos.chunk.ack"
            );
            if event.delivery.is_transport() && chunk_index >= event.chunks.len() {
                info!(
                    request_id = %key.request_id,
                    response_seq = key.seq,
                    stream_id = ?key.stream_id,
                    chunk_index,
                    "ble.qos.transport.final_range_ack"
                );
                state.events.remove(&key);
                return;
            }
            if let Some((command_name, next_index, chunk)) = next_transport_chunk(event) {
                let key = key.clone();
                let post_ack_notify_delay = self.post_ack_notify_delay;
                drop(state);
                self.send_chunk_after_ack(
                    key,
                    command_name,
                    next_index,
                    chunk,
                    chunk_index,
                    post_ack_notify_delay,
                );
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

    fn send_chunk(
        &self,
        key: &EventKey,
        command_name: &str,
        index: usize,
        chunk: &[u8],
        reason: SendReason,
    ) {
        let _ = self.tx.send(chunk.to_vec());
        info!(
            request_id = %key.request_id,
            cmd = %command_name,
            response_seq = key.seq,
            stream_id = ?key.stream_id,
            chunk_index = index,
            chunk_bytes = chunk.len(),
            send_reason = reason.as_str(),
            post_ack_delay_ms = reason.post_ack_delay_ms(),
            trigger_ack_index = reason.trigger_ack_index(),
            "ble.qos.chunk.sent"
        );
    }

    fn send_chunk_after_ack(
        &self,
        key: EventKey,
        command_name: String,
        index: usize,
        chunk: Vec<u8>,
        trigger_ack_index: usize,
        delay: Duration,
    ) {
        let sender = self.clone();
        tokio::spawn(async move {
            let delay_ms = delay.as_millis();
            if !delay.is_zero() {
                info!(
                    request_id = %key.request_id,
                    cmd = %command_name,
                    response_seq = key.seq,
                    stream_id = ?key.stream_id,
                    chunk_index = index,
                    trigger_ack_index,
                    post_ack_delay_ms = delay_ms,
                    "ble.qos.chunk.post_ack_wait"
                );
                tokio::time::sleep(delay).await;
            }
            if !sender.should_send_scheduled_chunk(&key, index).await {
                info!(
                    request_id = %key.request_id,
                    cmd = %command_name,
                    response_seq = key.seq,
                    stream_id = ?key.stream_id,
                    chunk_index = index,
                    trigger_ack_index,
                    post_ack_delay_ms = delay_ms,
                    "ble.qos.chunk.post_ack_skip"
                );
                return;
            }
            sender.send_chunk(
                &key,
                &command_name,
                index,
                &chunk,
                SendReason::PostAck {
                    delay_ms,
                    trigger_ack_index,
                },
            );
            sender
                .complete_transport_event_after_final_send(&key, index)
                .await;
        });
    }

    async fn should_send_scheduled_chunk(&self, key: &EventKey, index: usize) -> bool {
        let state = self.state.lock().await;
        state
            .events
            .get(key)
            .is_some_and(|event| !event.acked_chunks.contains(&index))
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

    async fn complete_transport_event_after_final_send(&self, key: &EventKey, index: usize) {
        let mut state = self.state.lock().await;
        let Some(event) = state.events.get(key) else {
            return;
        };
        if event.delivery.is_transport() && index >= event.chunks.len() {
            info!(
                request_id = %key.request_id,
                cmd = %event.command_name,
                response_seq = key.seq,
                stream_id = ?key.stream_id,
                chunk_index = index,
                "ble.qos.transport.final_sent"
            );
            state.events.remove(key);
        }
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
            let now = Instant::now();
            for index in retry_indexes {
                if event.acked_chunks.contains(&index) && !retry_whole_event {
                    continue;
                }
                if matches!(event.delivery, DeliveryMode::Transport { .. })
                    && event
                        .last_sent_at
                        .get(index - 1)
                        .and_then(|sent_at| *sent_at)
                        .is_some_and(|sent_at| now.duration_since(sent_at) < ACK_TIMEOUT)
                {
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
                if let Some(sent_at) = event.last_sent_at.get_mut(index - 1) {
                    *sent_at = Some(now);
                }
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

#[derive(Debug, Clone, Copy)]
enum SendReason {
    Initial,
    PostAck {
        delay_ms: u128,
        trigger_ack_index: usize,
    },
}

impl SendReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::PostAck { .. } => "post_ack",
        }
    }

    fn post_ack_delay_ms(self) -> Option<u128> {
        match self {
            Self::Initial => None,
            Self::PostAck { delay_ms, .. } => Some(delay_ms),
        }
    }

    fn trigger_ack_index(self) -> Option<usize> {
        match self {
            Self::Initial => None,
            Self::PostAck {
                trigger_ack_index, ..
            } => Some(trigger_ack_index),
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
        DeliveryMode::Transport { frame_budget, .. } => protocol::encode_response(resp)
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
    mark_chunk_sent(event, index);
    Some((
        event.command_name.clone(),
        index,
        event.chunks[index - 1].clone(),
    ))
}

fn mark_chunk_sent(event: &mut EventState, index: usize) {
    if let Some(slot) = event.last_sent_at.get_mut(index - 1) {
        *slot = Some(Instant::now());
    }
}

fn retry_indexes(event: &EventState) -> Vec<usize> {
    match event.delivery {
        DeliveryMode::LegacyJson => (1..=event.chunks.len()).collect(),
        DeliveryMode::Transport { .. } => (1..event.next_to_send)
            .filter(|index| !event.acked_chunks.contains(index))
            .collect(),
    }
}

fn configured_post_ack_notify_delay() -> Duration {
    std::env::var(POST_ACK_NOTIFY_DELAY_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_millis(DEFAULT_POST_ACK_NOTIFY_DELAY_MS))
}

#[cfg(test)]
mod tests;
