use std::collections::HashMap;

const MAGIC: u8 = 0x59;
const VERSION: u8 = 2;
const HEADER_LEN: usize = 4;
const FRAME_BUDGET: usize = 20;
const MIN_FRAME_LEN: usize = HEADER_LEN + 1;
pub const FRAME_HEADER_LEN: usize = HEADER_LEN;
pub const MAX_FRAME_PAYLOAD_LEN: usize = FRAME_BUDGET - HEADER_LEN;
pub const MAX_LOGICAL_PAYLOAD_LEN: usize = u8::MAX as usize * MAX_FRAME_PAYLOAD_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameKind {
    RequestChunk,
    RequestFinal,
    ResponseChunk,
    ResponseFinal,
    AckRange,
    AckEvent,
    Reset,
}

impl FrameKind {
    fn wire(self) -> u8 {
        match self {
            Self::RequestChunk => 1,
            Self::RequestFinal => 2,
            Self::ResponseChunk => 3,
            Self::ResponseFinal => 4,
            Self::AckRange => 5,
            Self::AckEvent => 6,
            Self::Reset => 7,
        }
    }

    fn from_wire(value: u8) -> Result<Self, TransportError> {
        match value {
            1 => Ok(Self::RequestChunk),
            2 => Ok(Self::RequestFinal),
            3 => Ok(Self::ResponseChunk),
            4 => Ok(Self::ResponseFinal),
            5 => Ok(Self::AckRange),
            6 => Ok(Self::AckEvent),
            7 => Ok(Self::Reset),
            _ => Err(TransportError::BadKind(value)),
        }
    }

    pub fn is_payload(self) -> bool {
        matches!(
            self,
            Self::RequestChunk | Self::RequestFinal | Self::ResponseChunk | Self::ResponseFinal
        )
    }

    pub fn is_ack(self) -> bool {
        matches!(self, Self::AckRange | Self::AckEvent)
    }

    pub fn final_frame(self) -> bool {
        matches!(self, Self::RequestFinal | Self::ResponseFinal)
    }

    fn payload_group(self) -> Result<PayloadGroup, TransportError> {
        match self {
            Self::RequestChunk | Self::RequestFinal => Ok(PayloadGroup::Request),
            Self::ResponseChunk | Self::ResponseFinal => Ok(PayloadGroup::Response),
            _ => Err(TransportError::BadKind(self.wire())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportFrame {
    pub kind: FrameKind,
    pub stream_id: u8,
    pub index: u8,
    pub final_frame: bool,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReassemblerEvent {
    pub frame: TransportFrame,
    pub duplicate: bool,
    pub payload: Option<Vec<u8>>,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    #[error("transport frame is too short")]
    TooShort,
    #[error("transport frame exceeds budget: {actual} > {budget}")]
    FrameTooLarge { actual: usize, budget: usize },
    #[error("transport frame budget leaves no payload room")]
    FrameBudgetTooSmall,
    #[error("transport magic mismatch")]
    BadMagic,
    #[error("unsupported transport version: {0}")]
    BadVersion(u8),
    #[error("unsupported transport kind: {0}")]
    BadKind(u8),
    #[error("payload is too large for one transport stream")]
    PayloadTooLarge,
    #[error("transport payload frame index must be non-zero")]
    BadPayloadIndex,
    #[error("ack frame must not contain payload")]
    AckHasPayload,
    #[error("transport final fragment index changed")]
    FinalIndexChanged,
    #[error("transport frame has empty payload")]
    EmptyPayload,
}

#[derive(Default)]
pub struct PayloadReassembler {
    streams: HashMap<StreamKey, StreamState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StreamKey {
    group: PayloadGroup,
    stream_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PayloadGroup {
    Request,
    Response,
}

struct StreamState {
    received: usize,
    final_index: Option<u8>,
    parts: Vec<Option<Vec<u8>>>,
}

impl PayloadReassembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept_frame(&mut self, raw: &[u8]) -> Result<ReassemblerEvent, TransportError> {
        let frame = decode_frame(raw)?;
        if !frame.kind.is_payload() {
            return Ok(ReassemblerEvent {
                frame,
                duplicate: false,
                payload: None,
            });
        }
        if frame.index == 0 {
            return Err(TransportError::BadPayloadIndex);
        }

        let key = StreamKey {
            group: frame.kind.payload_group()?,
            stream_id: frame.stream_id,
        };
        let state = self.streams.entry(key).or_insert_with(|| StreamState {
            received: 0,
            final_index: None,
            parts: vec![None; u8::MAX as usize],
        });
        if frame.final_frame {
            match state.final_index {
                Some(final_index) if final_index != frame.index => {
                    return Err(TransportError::FinalIndexChanged);
                }
                Some(_) => {}
                None => {
                    state.final_index = Some(frame.index);
                }
            }
        }

        let slot = &mut state.parts[usize::from(frame.index - 1)];
        let duplicate = slot.is_some();
        if !duplicate {
            *slot = Some(frame.payload.clone());
            state.received += 1;
        }

        let payload = if state
            .final_index
            .is_some_and(|final_index| state.received == usize::from(final_index))
        {
            let state = self.streams.remove(&key).expect("stream exists");
            let mut payload = Vec::new();
            for part in state
                .parts
                .into_iter()
                .take(usize::from(state.final_index.expect("final index set")))
            {
                payload.extend(part.expect("all parts are present"));
            }
            Some(payload)
        } else {
            None
        };

        Ok(ReassemblerEvent {
            frame,
            duplicate,
            payload,
        })
    }
}

pub fn is_transport_frame(raw: &[u8]) -> bool {
    raw.first().copied() == Some(MAGIC)
}

pub fn encode_payload_frames(
    kind: FrameKind,
    stream_id: u8,
    payload: &[u8],
    frame_budget: usize,
) -> Result<Vec<Vec<u8>>, TransportError> {
    let group = kind.payload_group()?;
    if !matches!(kind, FrameKind::RequestChunk | FrameKind::ResponseChunk) {
        return Err(TransportError::BadKind(kind.wire()));
    }
    if frame_budget < MIN_FRAME_LEN {
        return Err(TransportError::FrameBudgetTooSmall);
    }
    let payload_budget = (frame_budget - HEADER_LEN).min(MAX_FRAME_PAYLOAD_LEN);
    let total = payload.len().div_ceil(payload_budget);
    if total == 0 || total > u8::MAX as usize {
        return Err(TransportError::PayloadTooLarge);
    }

    payload
        .chunks(payload_budget)
        .enumerate()
        .map(|(index, chunk)| {
            let frame_index =
                u8::try_from(index + 1).map_err(|_| TransportError::PayloadTooLarge)?;
            let final_frame = index + 1 == total;
            let kind = match (group, final_frame) {
                (PayloadGroup::Request, false) => FrameKind::RequestChunk,
                (PayloadGroup::Request, true) => FrameKind::RequestFinal,
                (PayloadGroup::Response, false) => FrameKind::ResponseChunk,
                (PayloadGroup::Response, true) => FrameKind::ResponseFinal,
            };
            encode_frame(TransportFrame {
                kind,
                stream_id,
                index: frame_index,
                final_frame,
                payload: chunk.to_vec(),
            })
            .and_then(|frame| validate_budget(frame, frame_budget))
        })
        .collect()
}

pub fn encode_ack_frame(
    kind: FrameKind,
    stream_id: u8,
    index: u8,
    frame_budget: usize,
) -> Result<Vec<u8>, TransportError> {
    if !kind.is_ack() {
        return Err(TransportError::BadKind(kind.wire()));
    }
    validate_budget(
        encode_frame(TransportFrame {
            kind,
            stream_id,
            index,
            final_frame: false,
            payload: Vec::new(),
        })?,
        frame_budget,
    )
}

pub fn decode_frame(raw: &[u8]) -> Result<TransportFrame, TransportError> {
    if raw.len() < HEADER_LEN {
        return Err(TransportError::TooShort);
    }
    if raw.len() > FRAME_BUDGET {
        return Err(TransportError::FrameTooLarge {
            actual: raw.len(),
            budget: FRAME_BUDGET,
        });
    }
    if raw[0] != MAGIC {
        return Err(TransportError::BadMagic);
    }
    let version = raw[1] >> 4;
    if version != VERSION {
        return Err(TransportError::BadVersion(version));
    }
    let kind = FrameKind::from_wire(raw[1] & 0x0f)?;
    let payload = raw[HEADER_LEN..].to_vec();
    if kind.is_ack() && !payload.is_empty() {
        return Err(TransportError::AckHasPayload);
    }
    if kind.is_payload() {
        if raw[3] == 0 {
            return Err(TransportError::BadPayloadIndex);
        }
        if payload.is_empty() {
            return Err(TransportError::EmptyPayload);
        }
    }

    Ok(TransportFrame {
        kind,
        stream_id: raw[2],
        index: raw[3],
        final_frame: kind.final_frame(),
        payload,
    })
}

fn encode_frame(frame: TransportFrame) -> Result<Vec<u8>, TransportError> {
    if frame.kind.is_ack() && !frame.payload.is_empty() {
        return Err(TransportError::AckHasPayload);
    }
    if frame.kind.is_payload() {
        if frame.index == 0 {
            return Err(TransportError::BadPayloadIndex);
        }
        if frame.payload.is_empty() {
            return Err(TransportError::EmptyPayload);
        }
        if frame.payload.len() > MAX_FRAME_PAYLOAD_LEN {
            return Err(TransportError::FrameTooLarge {
                actual: HEADER_LEN + frame.payload.len(),
                budget: FRAME_BUDGET,
            });
        }
    }
    let mut raw = Vec::with_capacity(HEADER_LEN + frame.payload.len());
    raw.push(MAGIC);
    raw.push((VERSION << 4) | frame.kind.wire());
    raw.push(frame.stream_id);
    raw.push(frame.index);
    raw.extend(frame.payload);
    Ok(raw)
}

fn validate_budget(frame: Vec<u8>, frame_budget: usize) -> Result<Vec<u8>, TransportError> {
    if frame.len() > frame_budget {
        Err(TransportError::FrameTooLarge {
            actual: frame.len(),
            budget: frame_budget,
        })
    } else {
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_payload_fragments_round_trip_under_twenty_bytes() {
        let payload = br#"{"id":"req-1","cmd":"system.status","args":{},"v":"YundroneBT-V2.1.0"}"#;
        let frames = encode_payload_frames(FrameKind::RequestChunk, 7, payload, 20).unwrap();

        assert!(frames.len() > 1);
        assert!(frames.iter().all(|frame| frame.len() <= 20));

        let mut reassembler = PayloadReassembler::new();
        let mut completed = None;
        for raw in frames {
            completed = reassembler.accept_frame(&raw).unwrap().payload;
        }

        assert_eq!(completed.as_deref(), Some(payload.as_slice()));
    }

    #[test]
    fn v2_twenty_byte_frames_carry_sixteen_payload_bytes() {
        assert_eq!(FRAME_HEADER_LEN, 4);
        assert_eq!(MAX_FRAME_PAYLOAD_LEN, 16);

        let payload = [b'a'; 101];
        let frames = encode_payload_frames(FrameKind::RequestChunk, 7, &payload, 20).unwrap();

        assert_eq!(frames.len(), 7);
        assert!(frames.iter().all(|frame| frame.len() <= 20));
        assert!(frames.iter().take(6).all(|frame| frame.len() == 20));
        assert_eq!(frames.last().unwrap().len(), 9);
        assert_eq!(frames[0][0], 0x59);
        assert_eq!(frames[0][1] >> 4, 2);
        assert_eq!(frames[0][2], 7);
        assert_eq!(frames[0][3], 1);

        let first = decode_frame(&frames[0]).unwrap();
        let last = decode_frame(frames.last().unwrap()).unwrap();
        assert_eq!(first.payload.len(), 16);
        assert!(!first.final_frame);
        assert_eq!(last.index, 7);
        assert!(last.final_frame);
    }

    #[test]
    fn v2_response_payload_uses_thirty_seven_frames_for_591_bytes() {
        let payload = vec![b'x'; 591];
        let frames = encode_payload_frames(FrameKind::ResponseChunk, 9, &payload, 20).unwrap();

        assert_eq!(frames.len(), 37);
        assert_eq!(decode_frame(&frames[0]).unwrap().payload.len(), 16);
        assert!(decode_frame(frames.last().unwrap()).unwrap().final_frame);
    }

    #[test]
    fn v2_reassembler_accepts_final_first_out_of_order_and_duplicate_fragments() {
        let payload = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let frames = encode_payload_frames(FrameKind::RequestChunk, 9, payload, 20).unwrap();
        let mut reassembler = PayloadReassembler::new();

        assert!(reassembler
            .accept_frame(frames.last().unwrap())
            .unwrap()
            .payload
            .is_none());
        assert!(reassembler
            .accept_frame(&frames[1])
            .unwrap()
            .payload
            .is_none());
        assert!(reassembler
            .accept_frame(&frames[1])
            .unwrap()
            .payload
            .is_none());
        let completed = reassembler.accept_frame(&frames[0]).unwrap().payload;

        assert_eq!(completed.as_deref(), Some(payload.as_slice()));
    }

    #[test]
    fn v2_reassembler_rejects_conflicting_final_index() {
        let payload = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let frames = encode_payload_frames(FrameKind::RequestChunk, 9, payload, 20).unwrap();
        let mut conflicting_final = frames[1].clone();
        conflicting_final[1] = (2 << 4) | FrameKind::RequestFinal.wire();
        let mut reassembler = PayloadReassembler::new();

        reassembler.accept_frame(frames.last().unwrap()).unwrap();
        let err = reassembler.accept_frame(&conflicting_final).unwrap_err();

        assert_eq!(err, TransportError::FinalIndexChanged);
    }

    #[test]
    fn v2_rejects_zero_index_and_oversized_payload() {
        let zero_index = [0x59, (2 << 4) | FrameKind::RequestFinal.wire(), 1, 0, b'x'];
        let oversized = [
            vec![0x59, (2 << 4) | FrameKind::RequestFinal.wire(), 1, 1],
            vec![b'x'; 17],
        ]
        .concat();

        assert_eq!(
            decode_frame(&zero_index).unwrap_err(),
            TransportError::BadPayloadIndex
        );
        assert_eq!(
            decode_frame(&oversized).unwrap_err(),
            TransportError::FrameTooLarge {
                actual: 21,
                budget: 20
            }
        );
    }

    #[test]
    fn request_reassembler_accepts_out_of_order_and_duplicate_fragments() {
        let payload = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let frames = encode_payload_frames(FrameKind::RequestChunk, 9, payload, 20).unwrap();
        let mut reassembler = PayloadReassembler::new();

        assert!(reassembler
            .accept_frame(&frames[1])
            .unwrap()
            .payload
            .is_none());
        assert!(reassembler
            .accept_frame(&frames[1])
            .unwrap()
            .payload
            .is_none());
        assert!(reassembler
            .accept_frame(&frames[0])
            .unwrap()
            .payload
            .is_none());
        let completed = reassembler.accept_frame(&frames[2]).unwrap().payload;

        assert_eq!(completed.as_deref(), Some(payload.as_slice()));
    }

    #[test]
    fn v1_frames_are_rejected_after_v2_cutover() {
        let old_v1_frame = [
            0x59, 0x11, 0x00, 0x0b, 0x01, 0x01, 0x05, b'h', b'e', b'l', b'l', b'o',
        ];

        let err = decode_frame(&old_v1_frame).unwrap_err();

        assert_eq!(err, TransportError::BadVersion(1));
    }

    #[test]
    fn compact_ack_frames_decode_without_json_payload() {
        let raw = encode_ack_frame(FrameKind::AckRange, 42, 3, 20).unwrap();
        let frame = decode_frame(&raw).unwrap();

        assert_eq!(raw.len(), 4);
        assert_eq!(frame.kind, FrameKind::AckRange);
        assert_eq!(frame.stream_id, 42);
        assert_eq!(frame.index, 3);
        assert!(frame.payload.is_empty());
    }
}
