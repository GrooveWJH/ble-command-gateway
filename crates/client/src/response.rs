use std::collections::{HashMap, HashSet};

const TRANSPORT_RESPONSE_ACK_WINDOW: u8 = 1;

pub struct ResponseDecoder {
    assembler: protocol::chunking::ChunkAssembler,
}

pub struct TransportResponseDecoder {
    legacy: ResponseDecoder,
    transport: protocol::transport::PayloadReassembler,
    ranges: HashMap<u8, TransportRangeState>,
    completed: HashSet<CompletedTransportStream>,
}

#[derive(Default)]
struct TransportRangeState {
    received: HashSet<u8>,
    contiguous: u8,
    last_acked_contiguous: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CompletedTransportStream {
    stream_id: u8,
    final_index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkReceipt {
    pub response_id: String,
    pub response_seq: u64,
    pub chunk_index: usize,
    pub chunk_total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportAckType {
    Range,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportAckReceipt {
    pub stream_id: u8,
    pub ack_type: TransportAckType,
    pub index: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedEvent {
    pub chunk_receipt: Option<ChunkReceipt>,
    pub transport_ack: Option<TransportAckReceipt>,
    pub response: Option<protocol::CommandResponse>,
    pub transport_progress: bool,
    pub assembled_from_chunks: bool,
    pub assembled_from_transport: bool,
}

#[cfg(test)]
mod transport_tests {
    use super::*;

    #[test]
    fn transport_response_frames_reassemble_before_json_decode() {
        let response = protocol::CommandResponse::ok("req-transport", "x".repeat(160), None);
        let payload = protocol::encode_response(&response).unwrap();
        let frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::ResponseChunk,
            22,
            &payload,
            20,
        )
        .unwrap();
        let mut decoder = TransportResponseDecoder::new();
        let mut completed = None;

        for raw in frames {
            let event = decoder.decode_event(&raw).unwrap();
            if event.response.is_some() {
                completed = Some(event);
            }
        }

        let completed = completed.expect("transport response should complete");
        assert_eq!(completed.response, Some(response));
        assert_eq!(
            completed.transport_ack,
            Some(TransportAckReceipt {
                stream_id: 22,
                ack_type: TransportAckType::Event,
                index: 0,
            })
        );
        assert!(completed.assembled_from_transport);
    }

    #[test]
    fn transport_response_chunk_generates_compact_ack_receipt() {
        let response = protocol::CommandResponse::ok("req-transport", "x".repeat(160), None);
        let payload = protocol::encode_response(&response).unwrap();
        let frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::ResponseChunk,
            23,
            &payload,
            20,
        )
        .unwrap();
        let mut decoder = TransportResponseDecoder::new();

        let event = decoder.decode_event(&frames[0]).unwrap();

        assert!(event.response.is_none());
        assert_eq!(
            event.transport_ack,
            Some(TransportAckReceipt {
                stream_id: 23,
                ack_type: TransportAckType::Range,
                index: 1,
            })
        );
    }

    #[test]
    fn transport_response_sends_windowed_range_ack_for_contiguous_prefix() {
        let response = protocol::CommandResponse::ok("req-transport", "x".repeat(160), None);
        let payload = protocol::encode_response(&response).unwrap();
        let frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::ResponseChunk,
            25,
            &payload,
            20,
        )
        .unwrap();
        let mut decoder = TransportResponseDecoder::new();

        let first = decoder.decode_event(&frames[0]).unwrap();
        let second = decoder.decode_event(&frames[1]).unwrap();

        assert_eq!(
            first.transport_ack,
            Some(TransportAckReceipt {
                stream_id: 25,
                ack_type: TransportAckType::Range,
                index: 1,
            })
        );
        assert_eq!(
            second.transport_ack,
            Some(TransportAckReceipt {
                stream_id: 25,
                ack_type: TransportAckType::Range,
                index: 2,
            })
        );
    }

    #[test]
    fn transport_response_range_ack_does_not_advance_past_gap() {
        let response = protocol::CommandResponse::ok("req-transport", "x".repeat(160), None);
        let payload = protocol::encode_response(&response).unwrap();
        let frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::ResponseChunk,
            26,
            &payload,
            20,
        )
        .unwrap();
        let mut decoder = TransportResponseDecoder::new();

        let first = decoder.decode_event(&frames[0]).unwrap();
        let third = decoder.decode_event(&frames[2]).unwrap();

        assert_eq!(
            first.transport_ack,
            Some(TransportAckReceipt {
                stream_id: 26,
                ack_type: TransportAckType::Range,
                index: 1,
            })
        );
        assert_eq!(third.transport_ack, None);
    }

    #[test]
    fn transport_response_final_sends_event_ack_without_extra_range_ack() {
        let response = protocol::CommandResponse::ok("req-transport", "short", None);
        let payload = protocol::encode_response(&response).unwrap();
        let frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::ResponseChunk,
            27,
            &payload,
            20,
        )
        .unwrap();
        let mut decoder = TransportResponseDecoder::new();
        let mut completed = None;

        for raw in frames {
            let event = decoder.decode_event(&raw).unwrap();
            if event.response.is_some() {
                completed = Some(event);
            }
        }

        assert_eq!(
            completed.expect("response should complete").transport_ack,
            Some(TransportAckReceipt {
                stream_id: 27,
                ack_type: TransportAckType::Event,
                index: 0,
            })
        );
    }

    #[test]
    fn late_duplicate_final_does_not_poison_reused_stream() {
        let accepted = protocol::CommandResponse::accepted(
            "req-reuse".to_string(),
            "wifi.scan".to_string(),
            "accepted",
            None,
        );
        let mut result = protocol::CommandResponse::ok("req-reuse", "wifi scan complete", None);
        result.seq = 2;
        let accepted_frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::ResponseChunk,
            28,
            &protocol::encode_response(&accepted).unwrap(),
            20,
        )
        .unwrap();
        let result_frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::ResponseChunk,
            28,
            &protocol::encode_response(&result).unwrap(),
            20,
        )
        .unwrap();
        let mut decoder = TransportResponseDecoder::new();

        for raw in &accepted_frames {
            decoder.decode_event(raw).unwrap();
        }
        let duplicate_final = decoder
            .decode_event(accepted_frames.last().unwrap())
            .unwrap();
        assert_eq!(
            duplicate_final.transport_ack,
            Some(TransportAckReceipt {
                stream_id: 28,
                ack_type: TransportAckType::Event,
                index: 0,
            })
        );
        assert!(duplicate_final.response.is_none());

        let mut completed = None;
        for raw in &result_frames {
            let event = decoder.decode_event(raw).unwrap();
            if let Some(response) = event.response {
                completed = Some(response);
            }
        }

        assert_eq!(completed, Some(result));
    }

    #[test]
    fn transport_response_frame_marks_progress_before_reassembly() {
        let response = protocol::CommandResponse::ok("req-transport", "x".repeat(160), None);
        let payload = protocol::encode_response(&response).unwrap();
        let frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::ResponseChunk,
            24,
            &payload,
            20,
        )
        .unwrap();
        let mut decoder = TransportResponseDecoder::new();

        let event = decoder.decode_event(&frames[0]).unwrap();

        assert!(event.transport_progress);
        assert!(event.response.is_none());
    }

    #[test]
    fn transport_progress_control_frame_marks_progress_without_ack_or_json_decode() {
        let raw = protocol::transport::encode_control_frame(
            protocol::transport::FrameKind::Progress,
            0,
            2,
            20,
        )
        .unwrap();
        let mut decoder = TransportResponseDecoder::new();

        let event = decoder.decode_event(&raw).unwrap();

        assert!(event.transport_progress);
        assert!(event.response.is_none());
        assert_eq!(event.transport_ack, None);
        assert!(!event.assembled_from_transport);
    }
}

impl Default for ResponseDecoder {
    fn default() -> Self {
        Self {
            assembler: protocol::chunking::ChunkAssembler::new(),
        }
    }
}

impl ResponseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode(
        &mut self,
        raw: &[u8],
    ) -> Result<Option<protocol::CommandResponse>, protocol::ProtocolError> {
        self.decode_event(raw).map(|event| event.response)
    }

    pub fn decode_event(&mut self, raw: &[u8]) -> Result<DecodedEvent, protocol::ProtocolError> {
        let response = protocol::parse_response(raw)?;
        let chunk_receipt = chunk_receipt(&response);
        let assembled_from_chunks = chunk_receipt.is_some();
        let response = self.assembler.add_chunk(response)?;
        Ok(DecodedEvent {
            chunk_receipt,
            transport_ack: None,
            response,
            transport_progress: false,
            assembled_from_chunks,
            assembled_from_transport: false,
        })
    }
}

impl Default for TransportResponseDecoder {
    fn default() -> Self {
        Self {
            legacy: ResponseDecoder::new(),
            transport: protocol::transport::PayloadReassembler::new(),
            ranges: HashMap::new(),
            completed: HashSet::new(),
        }
    }
}

impl TransportResponseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode_event(&mut self, raw: &[u8]) -> Result<DecodedEvent, DecodeError> {
        if !protocol::transport::is_transport_frame(raw) {
            return self.legacy.decode_event(raw).map_err(DecodeError::Protocol);
        }

        let frame = protocol::transport::decode_frame(raw).map_err(DecodeError::Transport)?;
        if frame.kind == protocol::transport::FrameKind::Progress {
            return Ok(DecodedEvent {
                chunk_receipt: None,
                transport_ack: None,
                response: None,
                transport_progress: true,
                assembled_from_chunks: false,
                assembled_from_transport: false,
            });
        }

        match decoded_transport_frame(raw)? {
            CompletedFrameCheck::LateDuplicateFinal(completed)
                if self.completed.contains(&completed) =>
            {
                return Ok(DecodedEvent {
                    chunk_receipt: None,
                    transport_ack: Some(TransportAckReceipt {
                        stream_id: completed.stream_id,
                        ack_type: TransportAckType::Event,
                        index: 0,
                    }),
                    response: None,
                    transport_progress: true,
                    assembled_from_chunks: false,
                    assembled_from_transport: false,
                });
            }
            CompletedFrameCheck::NewResponseStream(stream_id) => {
                self.completed
                    .retain(|completed| completed.stream_id != stream_id);
            }
            _ => {}
        }

        let event = self
            .transport
            .accept_frame(raw)
            .map_err(DecodeError::Transport)?;
        let ack = self.transport_ack_for_event(&event);
        let response = match event.payload {
            Some(payload) => {
                Some(protocol::parse_response(&payload).map_err(DecodeError::Protocol)?)
            }
            None => None,
        };

        Ok(DecodedEvent {
            chunk_receipt: None,
            transport_ack: ack,
            response,
            transport_progress: matches!(
                event.frame.kind,
                protocol::transport::FrameKind::ResponseChunk
                    | protocol::transport::FrameKind::ResponseFinal
            ),
            assembled_from_chunks: false,
            assembled_from_transport: true,
        })
    }

    fn transport_ack_for_event(
        &mut self,
        event: &protocol::transport::ReassemblerEvent,
    ) -> Option<TransportAckReceipt> {
        match event.frame.kind {
            protocol::transport::FrameKind::ResponseFinal if event.payload.is_some() => {
                self.ranges.remove(&event.frame.stream_id);
                self.completed.insert(CompletedTransportStream {
                    stream_id: event.frame.stream_id,
                    final_index: event.frame.index,
                });
                Some(TransportAckReceipt {
                    stream_id: event.frame.stream_id,
                    ack_type: TransportAckType::Event,
                    index: 0,
                })
            }
            protocol::transport::FrameKind::ResponseChunk
            | protocol::transport::FrameKind::ResponseFinal => {
                let state = self.ranges.entry(event.frame.stream_id).or_default();
                state.received.insert(event.frame.index);
                while state.received.contains(&state.contiguous.saturating_add(1)) {
                    state.contiguous = state.contiguous.saturating_add(1);
                }
                let advanced = state.contiguous.saturating_sub(state.last_acked_contiguous);
                if advanced < TRANSPORT_RESPONSE_ACK_WINDOW || state.contiguous != event.frame.index
                {
                    return None;
                }
                state.last_acked_contiguous = state.contiguous;
                Some(TransportAckReceipt {
                    stream_id: event.frame.stream_id,
                    ack_type: TransportAckType::Range,
                    index: state.contiguous,
                })
            }
            _ => None,
        }
    }
}

enum CompletedFrameCheck {
    LateDuplicateFinal(CompletedTransportStream),
    NewResponseStream(u8),
    Other,
}

fn decoded_transport_frame(raw: &[u8]) -> Result<CompletedFrameCheck, DecodeError> {
    let frame = protocol::transport::decode_frame(raw).map_err(DecodeError::Transport)?;
    match frame.kind {
        protocol::transport::FrameKind::ResponseFinal => Ok(
            CompletedFrameCheck::LateDuplicateFinal(CompletedTransportStream {
                stream_id: frame.stream_id,
                final_index: frame.index,
            }),
        ),
        protocol::transport::FrameKind::ResponseChunk if frame.index == 1 => {
            Ok(CompletedFrameCheck::NewResponseStream(frame.stream_id))
        }
        _ => Ok(CompletedFrameCheck::Other),
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DecodeError {
    #[error(transparent)]
    Protocol(#[from] protocol::ProtocolError),
    #[error(transparent)]
    Transport(#[from] protocol::transport::TransportError),
}

fn chunk_receipt(response: &protocol::CommandResponse) -> Option<ChunkReceipt> {
    let chunk = response
        .data
        .as_ref()
        .and_then(|data| data.get("chunk"))
        .and_then(|value| value.as_object())?;
    let ack_required = chunk
        .get("ack_required")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if !ack_required {
        return None;
    }
    let index = chunk.get("index")?.as_u64()? as usize;
    let total = chunk.get("total")?.as_u64()? as usize;
    if index == 0 || total == 0 {
        return None;
    }
    Some(ChunkReceipt {
        response_id: response.id.clone(),
        response_seq: response.seq,
        chunk_index: index,
        chunk_total: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_unchunked_response() {
        let response = protocol::CommandResponse::ok("req-1", "ok", None);
        let payload = protocol::encode_response(&response).unwrap();
        let mut decoder = ResponseDecoder::new();

        let decoded = decoder.decode(&payload).unwrap().unwrap();

        assert_eq!(decoded, response);
    }

    #[test]
    fn decode_chunked_response_round_trip() {
        let response = protocol::CommandResponse::ok("req-2", "x".repeat(400), None);
        let chunks = protocol::chunking::chunk_response(response.clone());
        let mut decoder = ResponseDecoder::new();
        let mut assembled = None;

        for chunk in chunks {
            let payload = protocol::encode_response(&chunk).unwrap();
            assembled = decoder.decode(&payload).unwrap();
        }

        assert_eq!(assembled.unwrap(), response);
    }

    #[test]
    fn decode_chunked_large_data_response_round_trip() {
        let response_data = protocol::responses::WifiScanResponseData {
            ifname: Some("wlan0".to_string()),
            count: 18,
            networks: (0..18)
                .map(|index| protocol::responses::WifiNetwork {
                    ssid: format!("MeshNode-{index:02}-Backhaul-SSID"),
                    channel: ((index % 11) + 1).to_string(),
                    signal: 80 - index,
                })
                .collect(),
        };
        let response = protocol::CommandResponse::ok(
            "req-3",
            "wifi scan complete",
            Some(protocol::responses::to_map(&response_data).unwrap()),
        );
        let chunks = protocol::chunking::chunk_response(response.clone());
        let mut decoder = ResponseDecoder::new();
        let mut assembled = None;

        assert!(chunks.len() > 1);

        for chunk in chunks {
            let payload = protocol::encode_response(&chunk).unwrap();
            assembled = decoder.decode(&payload).unwrap();
        }

        let assembled = assembled.expect("decoder should reassemble large data response");
        let decoded_data: protocol::responses::WifiScanResponseData =
            assembled.decode_data().unwrap();
        assert_eq!(assembled, response);
        assert_eq!(decoded_data, response_data);
    }

    #[test]
    fn decode_event_reports_chunk_receipt_before_assembly() {
        let response = protocol::CommandResponse::ok("req-4", "x".repeat(500), None);
        let chunks = protocol::chunking::chunk_response(response);
        let payload = protocol::encode_response(&chunks[0]).unwrap();
        let mut decoder = ResponseDecoder::new();

        let event = decoder.decode_event(&payload).unwrap();

        assert_eq!(
            event.chunk_receipt,
            Some(ChunkReceipt {
                response_id: "req-4".to_string(),
                response_seq: 1,
                chunk_index: 1,
                chunk_total: chunks.len(),
            })
        );
        assert!(event.response.is_none());
        assert!(event.assembled_from_chunks);
    }

    #[test]
    fn decode_event_marks_plain_response_as_not_chunk_assembled() {
        let response = protocol::CommandResponse::ok("req-plain", "ok", None);
        let payload = protocol::encode_response(&response).unwrap();
        let mut decoder = ResponseDecoder::new();

        let event = decoder.decode_event(&payload).unwrap();

        assert_eq!(event.response, Some(response));
        assert!(!event.assembled_from_chunks);
    }

    #[test]
    fn decode_event_marks_completed_chunk_response_as_chunk_assembled() {
        let response = protocol::CommandResponse::ok("req-chunked", "x".repeat(500), None);
        let chunks = protocol::chunking::chunk_response(response);
        let mut decoder = ResponseDecoder::new();
        let mut final_event = None;

        for chunk in chunks {
            let payload = protocol::encode_response(&chunk).unwrap();
            let event = decoder.decode_event(&payload).unwrap();
            if event.response.is_some() {
                final_event = Some(event);
            }
        }

        assert!(
            final_event
                .expect("chunked response should complete")
                .assembled_from_chunks
        );
    }

    #[test]
    fn duplicate_chunk_does_not_prevent_assembly() {
        let response = protocol::CommandResponse::ok("req-5", "x".repeat(500), None);
        let chunks = protocol::chunking::chunk_response(response.clone());
        let mut decoder = ResponseDecoder::new();
        let first = protocol::encode_response(&chunks[0]).unwrap();
        assert!(decoder.decode(&first).unwrap().is_none());
        assert!(decoder.decode(&first).unwrap().is_none());

        let mut assembled = None;
        for chunk in chunks.iter().skip(1) {
            let payload = protocol::encode_response(chunk).unwrap();
            assembled = decoder.decode(&payload).unwrap();
        }

        assert_eq!(assembled.unwrap(), response);
    }
}
