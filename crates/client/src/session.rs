use anyhow::{anyhow, Result};
use btleplug::api::{CharPropFlags, Characteristic, Peripheral as _, ValueNotification};
use btleplug::platform::Peripheral;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use tracing::info;

static NEXT_TRANSPORT_STREAM_ID: AtomicU8 = AtomicU8::new(1);

enum EventWaitOutcome {
    Response(protocol::CommandResponse),
    Progress,
}

pub struct BleSession {
    device_name: String,
    device_rssi: Option<i16>,
    device: Peripheral,
    write_char: Characteristic,
    read_char: Characteristic,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    response_decoder: crate::response::TransportResponseDecoder,
}

impl BleSession {
    pub(crate) async fn connect(
        device_name: String,
        device_rssi: Option<i16>,
        device: Peripheral,
    ) -> Result<Self> {
        info!(
            device_name = %device_name,
            rssi = ?device_rssi,
            "ble.session.connecting"
        );
        device.connect().await?;
        device.discover_services().await?;
        let chars = device.characteristics();

        let write_uuid = uuid::Uuid::parse_str("6E400002-B5A3-F393-E0A9-E50E24DCCA9E")?;
        let read_uuid = uuid::Uuid::parse_str("6E400003-B5A3-F393-E0A9-E50E24DCCA9E")?;

        let mut write_char = None;
        let mut read_char = None;

        for c in chars {
            if c.uuid == write_uuid && c.properties.contains(CharPropFlags::WRITE) {
                write_char = Some(c.clone());
            } else if c.uuid == read_uuid
                && c.properties
                    .intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE)
            {
                read_char = Some(c.clone());
            }
        }

        let write_char = write_char.ok_or_else(|| anyhow!("Write characteristic not found"))?;
        let read_char = read_char.ok_or_else(|| anyhow!("Read/Notify characteristic not found"))?;
        device.subscribe(&read_char).await?;
        let notifications = device.notifications().await?;

        info!(
            device_name = %device_name,
            rssi = ?device_rssi,
            "ble.session.ready"
        );

        Ok(Self {
            device_name,
            device_rssi,
            device,
            write_char,
            read_char,
            notifications,
            response_decoder: crate::response::TransportResponseDecoder::new(),
        })
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn device_rssi(&self) -> Option<i16> {
        self.device_rssi
    }

    pub async fn send_payload(&self, payload: &[u8]) -> Result<()> {
        crate::qos::write_payload(
            &self.device,
            &self.write_char,
            &self.device_name,
            self.device_rssi,
            payload,
        )
        .await
    }

    pub async fn send_request(&self, request: &crate::PreparedRequest) -> Result<()> {
        info!(
            device_name = %self.device_name,
            rssi = ?self.device_rssi,
            cmd = %request.request.payload.command_name(),
            request_id = %request.request.id,
            payload_bytes = request.bytes.len(),
            "ble.request.sent"
        );
        self.send_transport_payload(protocol::transport::FrameKind::RequestChunk, &request.bytes)
            .await
    }

    pub async fn send_request_traced(
        &self,
        request: &crate::PreparedRequest,
        trace_options: crate::trace::TraceOptions,
        trace: Option<crate::trace::TraceCallback>,
    ) -> Result<()> {
        let (bytes, redacted) = if trace_options.redact_secrets {
            crate::trace::redacted_payload(&request.bytes)
        } else {
            (request.bytes.clone(), false)
        };
        crate::trace::emit(
            &trace,
            crate::trace::TraceEvent::TxRaw {
                kind: crate::trace::TraceWriteKind::Request,
                bytes,
                redacted,
            },
        );
        info!(
            device_name = %self.device_name,
            rssi = ?self.device_rssi,
            cmd = %request.request.payload.command_name(),
            request_id = %request.request.id,
            payload_bytes = request.bytes.len(),
            "ble.request.sent"
        );
        crate::trace::emit(
            &trace,
            crate::trace::TraceEvent::QosTx {
                kind: crate::trace::TraceWriteKind::Request,
                bytes: request.bytes.len(),
                write: "with-response",
            },
        );
        match self
            .send_transport_payload_traced(
                protocol::transport::FrameKind::RequestChunk,
                &request.bytes,
                crate::trace::TraceWriteKind::Request,
                trace.clone(),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(err) => {
                crate::trace::emit(
                    &trace,
                    crate::trace::TraceEvent::QosFallback {
                        kind: crate::trace::TraceWriteKind::Request,
                        reason: err.to_string(),
                    },
                );
                Err(err)
            }
        }
    }

    pub async fn next_response(&mut self, timeout_secs: u64) -> Result<protocol::CommandResponse> {
        self.next_event(timeout_secs).await
    }

    pub async fn next_event(&mut self, timeout_secs: u64) -> Result<protocol::CommandResponse> {
        self.next_event_traced(timeout_secs, None).await
    }

    pub async fn next_event_traced(
        &mut self,
        timeout_secs: u64,
        trace: Option<crate::trace::TraceCallback>,
    ) -> Result<protocol::CommandResponse> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(anyhow!(
                    "Timed out waiting for BLE response after {}s",
                    timeout_secs
                ));
            }
            match self
                .next_event_with_progress_for(remaining, trace.clone())
                .await?
            {
                EventWaitOutcome::Response(response) => return Ok(response),
                EventWaitOutcome::Progress => continue,
            }
        }
    }

    async fn next_event_with_progress(
        &mut self,
        timeout_secs: u64,
        trace: Option<crate::trace::TraceCallback>,
    ) -> Result<EventWaitOutcome> {
        self.next_event_with_progress_for(Duration::from_secs(timeout_secs), trace)
            .await
            .map_err(|err| {
                if err
                    .to_string()
                    .starts_with("Timed out waiting for BLE response")
                {
                    anyhow!("Timed out waiting for BLE response after {}s", timeout_secs)
                } else {
                    err
                }
            })
    }

    async fn next_event_with_progress_for(
        &mut self,
        timeout: Duration,
        trace: Option<crate::trace::TraceCallback>,
    ) -> Result<EventWaitOutcome> {
        tokio::time::timeout(timeout, async {
            while let Some(notification) = self.notifications.next().await {
                if notification.uuid != self.read_char.uuid {
                    continue;
                }

                for event in crate::trace::response_trace_events(&notification.value) {
                    crate::trace::emit(&trace, event);
                }
                match self.response_decoder.decode_event(&notification.value) {
                    Ok(event) => {
                        if let Some(ack) = &event.transport_ack {
                            let ack_bytes =
                                self.send_transport_ack_traced(ack, trace.clone()).await?;
                            crate::trace::emit(
                                &trace,
                                crate::trace::TraceEvent::QosTx {
                                    kind: crate::trace::TraceWriteKind::TransportAck,
                                    bytes: ack_bytes,
                                    write: "with-response",
                                },
                            );
                        }
                        let transport_progress = event.transport_progress;
                        if let Some(receipt) = &event.chunk_receipt {
                            if let Some(ack_bytes) = trace_link_ack(
                                &trace,
                                &receipt.response_id,
                                protocol::requests::LinkAckArgs {
                                    ack_type: protocol::requests::AckType::Chunk,
                                    response_seq: receipt.response_seq,
                                    chunk_index: Some(receipt.chunk_index),
                                },
                                crate::trace::TraceWriteKind::ChunkAck,
                            )? {
                                crate::trace::emit(
                                    &trace,
                                    crate::trace::TraceEvent::QosTx {
                                        kind: crate::trace::TraceWriteKind::ChunkAck,
                                        bytes: ack_bytes,
                                        write: "with-response",
                                    },
                                );
                            };
                            crate::qos::send_chunk_ack(
                                &self.device,
                                &self.write_char,
                                &self.device_name,
                                self.device_rssi,
                                receipt,
                            )
                            .await?;
                            crate::trace::emit(
                                &trace,
                                crate::trace::TraceEvent::QosChunkAck {
                                    response_id: receipt.response_id.clone(),
                                    response_seq: receipt.response_seq,
                                    chunk_index: receipt.chunk_index,
                                    chunk_total: receipt.chunk_total,
                                },
                            );
                        }
                        let response_acknowledged_by_transport = matches!(
                            event.transport_ack.as_ref().map(|ack| ack.ack_type),
                            Some(crate::response::TransportAckType::Event)
                        );
                        let Some(response) = event.response else {
                            if transport_progress {
                                return Ok(EventWaitOutcome::Progress);
                            }
                            continue;
                        };
                        if !response_acknowledged_by_transport {
                            if let Some(ack_bytes) = trace_link_ack(
                                &trace,
                                &response.id,
                                protocol::requests::LinkAckArgs {
                                    ack_type: protocol::requests::AckType::Event,
                                    response_seq: response.seq,
                                    chunk_index: None,
                                },
                                crate::trace::TraceWriteKind::EventAck,
                            )? {
                                crate::trace::emit(
                                    &trace,
                                    crate::trace::TraceEvent::QosTx {
                                        kind: crate::trace::TraceWriteKind::EventAck,
                                        bytes: ack_bytes,
                                        write: "with-response",
                                    },
                                );
                            };
                            crate::qos::send_event_ack(
                                &self.device,
                                &self.write_char,
                                &self.device_name,
                                self.device_rssi,
                                &response,
                            )
                            .await?;
                            crate::trace::emit(
                                &trace,
                                crate::trace::TraceEvent::QosEventAck {
                                    response_id: response.id.clone(),
                                    response_seq: response.seq,
                                },
                            );
                        }
                        if event.assembled_from_chunks || event.assembled_from_transport {
                            if let Ok(bytes) = protocol::encode_response(&response) {
                                crate::trace::emit(
                                    &trace,
                                    crate::trace::TraceEvent::RxAssembled { bytes },
                                );
                            }
                        }
                        info!(
                            device_name = %self.device_name,
                            rssi = ?self.device_rssi,
                            response_id = %response.id,
                            ok = response.ok,
                            code = %response.code,
                            "ble.response.received"
                        );
                        return Ok(EventWaitOutcome::Response(response));
                    }
                    Err(err) => return Err(anyhow!(err.to_string())),
                }
            }

            Err(anyhow!(
                "Notification stream closed before a response was received"
            ))
        })
        .await
        .map_err(|_| anyhow!("Timed out waiting for BLE response"))?
    }

    async fn send_transport_payload(
        &self,
        kind: protocol::transport::FrameKind,
        payload: &[u8],
    ) -> Result<()> {
        let stream_id = next_transport_stream_id();
        let frames = protocol::transport::encode_payload_frames(kind, stream_id, payload, 20)
            .map_err(|err| anyhow!(err.to_string()))?;
        for frame in frames {
            self.send_payload(&frame).await?;
        }
        Ok(())
    }

    async fn send_transport_payload_traced(
        &self,
        kind: protocol::transport::FrameKind,
        payload: &[u8],
        write_kind: crate::trace::TraceWriteKind,
        trace: Option<crate::trace::TraceCallback>,
    ) -> Result<()> {
        let stream_id = next_transport_stream_id();
        let frames = protocol::transport::encode_payload_frames(kind, stream_id, payload, 20)
            .map_err(|err| anyhow!(err.to_string()))?;
        for frame in frames {
            crate::trace::emit(
                &trace,
                crate::trace::TraceEvent::TxPacket {
                    kind: write_kind,
                    bytes: frame.clone(),
                },
            );
            self.send_payload(&frame).await?;
        }
        Ok(())
    }

    async fn send_transport_ack_traced(
        &self,
        ack: &crate::response::TransportAckReceipt,
        trace: Option<crate::trace::TraceCallback>,
    ) -> Result<usize> {
        let kind = match ack.ack_type {
            crate::response::TransportAckType::Range => protocol::transport::FrameKind::AckRange,
            crate::response::TransportAckType::Event => protocol::transport::FrameKind::AckEvent,
        };
        let frame = protocol::transport::encode_ack_frame(kind, ack.stream_id, ack.index, 20)
            .map_err(|err| anyhow!(err.to_string()))?;
        let len = frame.len();
        crate::trace::emit(
            &trace,
            crate::trace::TraceEvent::TxPacket {
                kind: crate::trace::TraceWriteKind::TransportAck,
                bytes: frame.clone(),
            },
        );
        self.send_payload(&frame).await?;
        Ok(len)
    }

    pub async fn run_request_until_final<F>(
        &mut self,
        request: &crate::PreparedRequest,
        timeout_secs: u64,
        mut on_event: F,
    ) -> Result<protocol::CommandResponse>
    where
        F: FnMut(&protocol::CommandResponse),
    {
        let first_response = self.send_request_reliably(request, timeout_secs).await?;
        if let Some(response) = first_response {
            let matches_request = response.id == request.request.id;
            on_event(&response);
            if matches_request && response.final_flag {
                return Ok(response);
            }
        }
        loop {
            let response = self.next_event(timeout_secs).await?;
            let matches_request = response.id == request.request.id;
            on_event(&response);
            if matches_request && response.final_flag {
                return Ok(response);
            }
        }
    }

    pub async fn run_request_until_final_traced<F>(
        &mut self,
        request: &crate::PreparedRequest,
        timeout_secs: u64,
        trace_options: crate::trace::TraceOptions,
        trace: Option<crate::trace::TraceCallback>,
        mut on_event: F,
    ) -> Result<protocol::CommandResponse>
    where
        F: FnMut(&protocol::CommandResponse),
    {
        let first_response = self
            .send_request_reliably_traced(request, timeout_secs, trace_options, trace.clone())
            .await?;
        if let Some(response) = first_response {
            let matches_request = response.id == request.request.id;
            on_event(&response);
            if matches_request && response.final_flag {
                return Ok(response);
            }
        }
        loop {
            let response = self.next_event_traced(timeout_secs, trace.clone()).await?;
            let matches_request = response.id == request.request.id;
            on_event(&response);
            if matches_request && response.final_flag {
                return Ok(response);
            }
        }
    }

    async fn send_request_reliably(
        &mut self,
        request: &crate::PreparedRequest,
        timeout_secs: u64,
    ) -> Result<Option<protocol::CommandResponse>> {
        let mut last_error = None;
        for attempt in 1..=crate::qos::REQUEST_ACCEPT_RETRIES {
            self.send_request(request).await?;
            let wait_secs = crate::qos::REQUEST_ACCEPT_TIMEOUT_SECS.min(timeout_secs);
            match self.next_event_with_progress(wait_secs, None).await {
                Ok(EventWaitOutcome::Progress) => {
                    info!(
                        device_name = %self.device_name,
                        rssi = ?self.device_rssi,
                        request_id = %request.request.id,
                        attempt,
                        "qos.request.transport_progress"
                    );
                    return Ok(None);
                }
                Ok(EventWaitOutcome::Response(response)) if response.id == request.request.id => {
                    info!(
                        device_name = %self.device_name,
                        rssi = ?self.device_rssi,
                        request_id = %request.request.id,
                        attempt,
                        phase = ?response.phase,
                        "qos.request.accepted"
                    );
                    return Ok(Some(response));
                }
                Ok(EventWaitOutcome::Response(response)) => {
                    info!(
                        device_name = %self.device_name,
                        rssi = ?self.device_rssi,
                        request_id = %request.request.id,
                        response_id = %response.id,
                        "qos.request.ignored_other_response"
                    );
                }
                Err(err) => {
                    info!(
                        device_name = %self.device_name,
                        rssi = ?self.device_rssi,
                        request_id = %request.request.id,
                        attempt,
                        error = %err,
                        "qos.request.retry"
                    );
                    last_error = Some(err);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow!("request was not accepted")))
    }

    async fn send_request_reliably_traced(
        &mut self,
        request: &crate::PreparedRequest,
        timeout_secs: u64,
        trace_options: crate::trace::TraceOptions,
        trace: Option<crate::trace::TraceCallback>,
    ) -> Result<Option<protocol::CommandResponse>> {
        let mut last_error = None;
        for attempt in 1..=crate::qos::REQUEST_ACCEPT_RETRIES {
            self.send_request_traced(request, trace_options, trace.clone())
                .await?;
            let wait_secs = crate::qos::REQUEST_ACCEPT_TIMEOUT_SECS.min(timeout_secs);
            match self
                .next_event_with_progress(wait_secs, trace.clone())
                .await
            {
                Ok(EventWaitOutcome::Progress) => {
                    info!(
                        device_name = %self.device_name,
                        rssi = ?self.device_rssi,
                        request_id = %request.request.id,
                        attempt,
                        "qos.request.transport_progress"
                    );
                    return Ok(None);
                }
                Ok(EventWaitOutcome::Response(response)) if response.id == request.request.id => {
                    info!(
                        device_name = %self.device_name,
                        rssi = ?self.device_rssi,
                        request_id = %request.request.id,
                        attempt,
                        phase = ?response.phase,
                        "qos.request.accepted"
                    );
                    return Ok(Some(response));
                }
                Ok(EventWaitOutcome::Response(response)) => {
                    info!(
                        device_name = %self.device_name,
                        rssi = ?self.device_rssi,
                        request_id = %request.request.id,
                        response_id = %response.id,
                        "qos.request.ignored_other_response"
                    );
                }
                Err(err) => {
                    crate::trace::emit(
                        &trace,
                        crate::trace::TraceEvent::RequestRetry {
                            request_id: request.request.id.clone(),
                            attempt,
                            error: err.to_string(),
                        },
                    );
                    info!(
                        device_name = %self.device_name,
                        rssi = ?self.device_rssi,
                        request_id = %request.request.id,
                        attempt,
                        error = %err,
                        "qos.request.retry"
                    );
                    last_error = Some(err);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow!("request was not accepted")))
    }

    pub async fn disconnect(&self) -> Result<()> {
        if let Err(err) = self.device.unsubscribe(&self.read_char).await {
            if !is_already_disconnected_error(&err.to_string()) {
                return Err(err.into());
            }
        }

        match self.device.is_connected().await {
            Ok(false) => Ok(()),
            Ok(true) => self.device.disconnect().await.map_err(Into::into),
            Err(err) if is_already_disconnected_error(&err.to_string()) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

fn next_transport_stream_id() -> u8 {
    let id = NEXT_TRANSPORT_STREAM_ID.fetch_add(1, Ordering::Relaxed);
    if id == 0 {
        NEXT_TRANSPORT_STREAM_ID.fetch_add(1, Ordering::Relaxed)
    } else {
        id
    }
}

fn is_already_disconnected_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("not connected")
        || lower.contains("already disconnected")
        || lower.contains("peripheral disconnected")
}

fn trace_link_ack(
    trace: &Option<crate::trace::TraceCallback>,
    request_id: &str,
    ack: protocol::requests::LinkAckArgs,
    kind: crate::trace::TraceWriteKind,
) -> Result<Option<usize>> {
    if trace.is_none() {
        return Ok(None);
    }
    let request = protocol::CommandRequest::new(
        request_id.to_string(),
        protocol::requests::CommandPayload::LinkAck(ack),
    );
    let bytes = protocol::encode_request(&request).map_err(|err| anyhow!(err.to_string()))?;
    let len = bytes.len();
    crate::trace::emit(
        trace,
        crate::trace::TraceEvent::TxRaw {
            kind,
            bytes,
            redacted: false,
        },
    );
    Ok(Some(len))
}

#[cfg(test)]
mod tests {
    use super::is_already_disconnected_error;

    #[test]
    fn known_disconnect_errors_are_tolerated() {
        assert!(is_already_disconnected_error("Peripheral is not connected"));
        assert!(is_already_disconnected_error("already disconnected"));
        assert!(is_already_disconnected_error(
            "peripheral disconnected by peer"
        ));
    }

    #[test]
    fn unrelated_errors_are_not_treated_as_disconnects() {
        assert!(!is_already_disconnected_error("permission denied"));
        assert!(!is_already_disconnected_error(
            "write characteristic missing"
        ));
    }

    #[test]
    fn event_wait_outcome_progress_is_not_terminal_response() {
        assert!(!matches!(
            super::EventWaitOutcome::Progress,
            super::EventWaitOutcome::Response(_)
        ));
    }
}
