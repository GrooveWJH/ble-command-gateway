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

#[derive(Clone)]
pub struct ReliableEventSender {
    tx: broadcast::Sender<Vec<u8>>,
    state: Arc<Mutex<ReliableState>>,
}

#[derive(Default)]
struct ReliableState {
    events: HashMap<EventKey, EventState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EventKey {
    request_id: String,
    seq: u64,
}

struct EventState {
    command_name: String,
    chunks: Vec<Vec<u8>>,
    acked_chunks: HashSet<usize>,
    retry_counts: Vec<u8>,
    event_acked: bool,
}

impl ReliableEventSender {
    pub fn new(tx: broadcast::Sender<Vec<u8>>) -> Self {
        Self {
            tx,
            state: Arc::new(Mutex::new(ReliableState::default())),
        }
    }

    pub async fn send_event(&self, resp: protocol::CommandResponse, command_name: &str) {
        let chunks = encode_chunks(&resp);
        let key = EventKey {
            request_id: resp.id.clone(),
            seq: resp.seq,
        };
        {
            let mut state = self.state.lock().await;
            if state.events.len() >= MAX_EVENTS {
                if let Some(first_key) = state.events.keys().next().cloned() {
                    state.events.remove(&first_key);
                    warn!(
                        request_id = %first_key.request_id,
                        response_seq = first_key.seq,
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
                },
            );
        }

        for (index, chunk) in chunks.iter().enumerate() {
            self.send_chunk(&key, command_name, index + 1, chunk);
        }

        self.spawn_retry_loop(key);
    }

    pub async fn ack_chunk(&self, request_id: &str, response_seq: u64, chunk_index: usize) {
        let key = EventKey {
            request_id: request_id.to_string(),
            seq: response_seq,
        };
        let mut state = self.state.lock().await;
        if let Some(event) = state.events.get_mut(&key) {
            event.acked_chunks.insert(chunk_index);
            info!(request_id, response_seq, chunk_index, "ble.qos.chunk.ack");
        }
    }

    pub async fn ack_event(&self, request_id: &str, response_seq: u64) {
        let key = EventKey {
            request_id: request_id.to_string(),
            seq: response_seq,
        };
        let mut state = self.state.lock().await;
        if let Some(event) = state.events.get_mut(&key) {
            event.event_acked = true;
            info!(request_id, response_seq, "ble.qos.event.ack");
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
            let retry_whole_event = !event.chunks.is_empty()
                && (1..=event.chunks.len()).all(|index| event.acked_chunks.contains(&index));
            for index in 1..=event.chunks.len() {
                if event.acked_chunks.contains(&index) && !retry_whole_event {
                    continue;
                }
                let retry_slot = &mut event.retry_counts[index - 1];
                if *retry_slot >= MAX_RETRIES {
                    warn!(
                        request_id = %key.request_id,
                        cmd = %event.command_name,
                        response_seq = key.seq,
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
                "ble.qos.delivery_failed"
            );
        }
    }
}

fn encode_chunks(resp: &protocol::CommandResponse) -> Vec<Vec<u8>> {
    protocol::chunking::chunk_response(resp.clone())
        .into_iter()
        .filter_map(|chunk| protocol::encode_response(&chunk).ok())
        .collect()
}

#[cfg(test)]
mod tests;
