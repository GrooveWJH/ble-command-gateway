use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{broadcast, Mutex};

#[derive(Clone)]
pub struct CommandEventSender {
    tx: crate::qos::ReliableEventSender,
    foreground_lock: Arc<Mutex<()>>,
    request_cache: Arc<Mutex<crate::request_cache::RequestCache>>,
    service_context: crate::services::ServiceContext,
}

impl CommandEventSender {
    pub fn new(
        tx: broadcast::Sender<Vec<u8>>,
        service_context: crate::services::ServiceContext,
    ) -> Self {
        Self {
            tx: crate::qos::ReliableEventSender::new(tx),
            foreground_lock: Arc::new(Mutex::new(())),
            request_cache: Arc::new(Mutex::new(crate::request_cache::RequestCache::new())),
            service_context,
        }
    }

    pub async fn handle_ack(&self, ack: protocol::requests::LinkAckArgs, request_id: &str) {
        match ack.ack_type {
            protocol::requests::AckType::Chunk => {
                if let Some(chunk_index) = ack.chunk_index {
                    self.tx
                        .ack_chunk(request_id, ack.response_seq, chunk_index)
                        .await;
                }
            }
            protocol::requests::AckType::Event => {
                self.tx.ack_event(request_id, ack.response_seq).await;
            }
        }
    }

    pub async fn handle_transport_ack(&self, ack: crate::transport::TransportAck) {
        self.tx
            .ack_transport(ack.kind, ack.stream_id, ack.index)
            .await;
    }

    pub async fn handle_request(&self, req: protocol::CommandRequest, command_name: String) {
        let cacheable = is_cacheable_request(&req.payload);
        if cacheable {
            if let Some(cached) = self.cached_response_for_duplicate(&req.id).await {
                self.send_response_event(cached, &command_name);
                return;
            }
            self.mark_request_started(&req.id, &command_name).await;
        }
        if is_long_running(&req.payload) {
            self.spawn_long_running(req, command_name);
        } else {
            let result =
                crate::services::run_payload_command(&self.service_context, &req.payload, 30.0)
                    .await;
            let response = result_response(&req, &command_name, 1, result);
            self.mark_request_final(&response).await;
            self.send_response_event(response, &command_name);
        }
    }

    pub async fn handle_transport_request(
        &self,
        req: protocol::CommandRequest,
        command_name: String,
        transport: crate::transport::TransportContext,
    ) {
        let cacheable = is_cacheable_request(&req.payload);
        if cacheable {
            if let Some(cached) = self.cached_response_for_duplicate(&req.id).await {
                self.send_response_event_with_delivery(
                    cached,
                    &command_name,
                    transport_delivery(transport),
                );
                return;
            }
            self.mark_request_started(&req.id, &command_name).await;
        }
        if is_long_running(&req.payload) {
            self.spawn_long_running_with_delivery(req, command_name, transport_delivery(transport));
        } else {
            let result =
                crate::services::run_payload_command(&self.service_context, &req.payload, 30.0)
                    .await;
            let response = result_response(&req, &command_name, 1, result);
            self.mark_request_final(&response).await;
            self.send_response_event_with_delivery(
                response,
                &command_name,
                transport_delivery(transport),
            );
        }
    }

    async fn cached_response_for_duplicate(
        &self,
        request_id: &str,
    ) -> Option<protocol::CommandResponse> {
        let mut cache = self.request_cache.lock().await;
        cache.duplicate_response(request_id)
    }

    async fn mark_request_started(&self, request_id: &str, command_name: &str) {
        let mut cache = self.request_cache.lock().await;
        cache.mark_started(request_id, command_name);
    }

    async fn mark_request_final(&self, response: &protocol::CommandResponse) {
        let mut cache = self.request_cache.lock().await;
        cache.mark_final(response);
    }

    fn spawn_long_running(&self, req: protocol::CommandRequest, command_name: String) {
        self.spawn_long_running_with_delivery(
            req,
            command_name,
            crate::qos::DeliveryMode::LegacyJson,
        );
    }

    fn spawn_long_running_with_delivery(
        &self,
        req: protocol::CommandRequest,
        command_name: String,
        delivery: crate::qos::DeliveryMode,
    ) {
        let tx = self.tx.clone();
        let lock = self.foreground_lock.clone();
        let cache = self.request_cache.clone();
        let service_context = self.service_context.clone();
        tokio::spawn(async move {
            let Ok(_guard) = lock.try_lock() else {
                let response = protocol::CommandResponse::result(
                    req.id,
                    Some(command_name.clone()),
                    false,
                    protocol::codes::CODE_BUSY,
                    "another foreground command is already running",
                    None,
                );
                {
                    let mut cache = cache.lock().await;
                    cache.mark_final(&response);
                }
                crate::response_events::send_response_event_with_delivery(
                    tx.clone(),
                    response,
                    &command_name,
                    delivery,
                )
                .await;
                return;
            };

            crate::response_events::send_response_event_with_delivery(
                tx.clone(),
                protocol::CommandResponse::accepted(
                    req.id.clone(),
                    command_name.clone(),
                    "accepted",
                    None,
                ),
                &command_name,
                delivery,
            )
            .await;

            let next_seq = Arc::new(AtomicU64::new(2));
            let progress_task = spawn_progress_loop(
                tx.clone(),
                req.id.clone(),
                command_name.clone(),
                next_seq.clone(),
                delivery,
            );
            let result =
                crate::services::run_payload_command(&service_context, &req.payload, 30.0).await;
            progress_task.abort();
            let final_seq = next_seq.fetch_add(1, Ordering::SeqCst);
            let response = result_response(&req, &command_name, final_seq, result);
            {
                let mut cache = cache.lock().await;
                cache.mark_final(&response);
            }

            crate::response_events::send_response_event_with_delivery(
                tx,
                response,
                &command_name,
                delivery,
            )
            .await;
        });
    }

    pub fn send_response_event(&self, resp: protocol::CommandResponse, command_name: &str) {
        if resp.final_flag {
            let this = self.clone();
            let response = resp.clone();
            tokio::spawn(async move {
                this.mark_request_final(&response).await;
            });
        }
        let tx = self.tx.clone();
        let command_name = command_name.to_string();
        tokio::spawn(async move {
            crate::response_events::send_response_event(tx, resp, &command_name).await;
        });
    }

    pub fn send_response_event_with_delivery(
        &self,
        resp: protocol::CommandResponse,
        command_name: &str,
        delivery: crate::qos::DeliveryMode,
    ) {
        if resp.final_flag {
            let this = self.clone();
            let response = resp.clone();
            tokio::spawn(async move {
                this.mark_request_final(&response).await;
            });
        }
        let tx = self.tx.clone();
        let command_name = command_name.to_string();
        tokio::spawn(async move {
            crate::response_events::send_response_event_with_delivery(
                tx,
                resp,
                &command_name,
                delivery,
            )
            .await;
        });
    }
}

fn spawn_progress_loop(
    tx: crate::qos::ReliableEventSender,
    request_id: String,
    command_name: String,
    next_seq: Arc<AtomicU64>,
    delivery: crate::qos::DeliveryMode,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let seq = next_seq.fetch_add(1, Ordering::SeqCst);
            crate::response_events::send_response_event_with_delivery(
                tx.clone(),
                protocol::CommandResponse::progress(
                    request_id.clone(),
                    command_name.clone(),
                    seq,
                    "please wait",
                    None,
                ),
                &command_name,
                delivery,
            )
            .await;
        }
    })
}

fn transport_delivery(context: crate::transport::TransportContext) -> crate::qos::DeliveryMode {
    let _ = context;
    crate::qos::DeliveryMode::Transport {
        frame_budget: crate::qos::TRANSPORT_FRAME_BUDGET,
        window_size: crate::qos::TRANSPORT_WINDOW_SIZE,
    }
}

pub fn is_long_running(payload: &protocol::requests::CommandPayload) -> bool {
    matches!(
        payload,
        protocol::requests::CommandPayload::WifiScan { .. }
            | protocol::requests::CommandPayload::WifiProvision { .. }
            | protocol::requests::CommandPayload::WifiProfilesDelete { .. }
    )
}

pub fn is_cacheable_request(payload: &protocol::requests::CommandPayload) -> bool {
    !matches!(
        payload,
        protocol::requests::CommandPayload::LinkAck(_)
            | protocol::requests::CommandPayload::LinkHeartbeat
    )
}

fn result_response(
    req: &protocol::CommandRequest,
    command_name: &str,
    seq: u64,
    result: crate::services::SystemExecResult,
) -> protocol::CommandResponse {
    let mut response = protocol::CommandResponse::result(
        req.id.clone(),
        Some(command_name.to_string()),
        result.ok,
        result.code,
        result.text,
        result.data,
    );
    response.seq = seq;
    response
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_slow_foreground_commands_emit_progress_events() {
        assert!(super::is_long_running(
            &protocol::requests::CommandPayload::WifiScan { ifname: None }
        ));
        assert!(super::is_long_running(
            &protocol::requests::CommandPayload::WifiProvision {
                ssid: "LabWiFi".to_string(),
                pwd: None,
            }
        ));
        assert!(super::is_long_running(
            &protocol::requests::CommandPayload::WifiProfilesDelete {
                uuids: vec!["uuid".to_string()],
                force: false,
            }
        ));
        assert!(!super::is_long_running(
            &protocol::requests::CommandPayload::LinkHeartbeat
        ));
        assert!(!super::is_long_running(
            &protocol::requests::CommandPayload::WifiProfilesList
        ));
    }

    #[test]
    fn heartbeat_and_ack_do_not_enter_request_retry_cache() {
        assert!(!super::is_cacheable_request(
            &protocol::requests::CommandPayload::LinkHeartbeat
        ));
        assert!(!super::is_cacheable_request(
            &protocol::requests::CommandPayload::LinkAck(protocol::requests::LinkAckArgs {
                ack_type: protocol::requests::AckType::Event,
                response_seq: 1,
                chunk_index: None,
            })
        ));
        assert!(super::is_cacheable_request(
            &protocol::requests::CommandPayload::WifiScan { ifname: None }
        ));
    }

    #[tokio::test]
    async fn transport_long_running_progress_uses_transport_delivery() {
        let (tx, mut rx) = tokio::sync::broadcast::channel(32);
        let sender = crate::qos::ReliableEventSender::new(tx);
        let next_seq = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(2));
        let task = super::spawn_progress_loop(
            sender,
            "req-progress".to_string(),
            "wifi.scan".to_string(),
            next_seq,
            crate::qos::DeliveryMode::Transport {
                frame_budget: 20,
                window_size: 1,
            },
        );

        let raw = tokio::time::timeout(std::time::Duration::from_millis(1200), rx.recv())
            .await
            .expect("progress frame should be sent")
            .unwrap();
        task.abort();

        let frame = protocol::transport::decode_frame(&raw).unwrap();
        assert_eq!(frame.kind, protocol::transport::FrameKind::Progress);
        assert_eq!(frame.index, 2);
        assert!(frame.payload.is_empty());
    }
}
