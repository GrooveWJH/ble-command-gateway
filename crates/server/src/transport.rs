#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportContext {
    pub stream_id: u8,
    pub final_frame_index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportAck {
    pub kind: protocol::transport::FrameKind,
    pub stream_id: u8,
    pub index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportRequestFrame {
    pub kind: protocol::transport::FrameKind,
    pub stream_id: u8,
    pub index: u8,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteEvent {
    Request {
        request: protocol::CommandRequest,
        transport: Option<TransportContext>,
    },
    TransportAck(TransportAck),
    Incomplete {
        transport: Option<TransportRequestFrame>,
    },
}

#[derive(Default)]
pub struct WriteRouter {
    reassembler: protocol::transport::PayloadReassembler,
}

impl WriteRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.reassembler = protocol::transport::PayloadReassembler::new();
    }

    pub fn accept_write(&mut self, raw: &[u8]) -> Result<WriteEvent, WriteRouterError> {
        if !protocol::transport::is_transport_frame(raw) {
            let request = protocol::parse_request(raw)?;
            return Ok(WriteEvent::Request {
                request,
                transport: None,
            });
        }

        let event = self.reassembler.accept_frame(raw)?;
        match event.frame.kind {
            protocol::transport::FrameKind::RequestChunk
            | protocol::transport::FrameKind::RequestFinal => {
                let Some(payload) = event.payload else {
                    return Ok(WriteEvent::Incomplete {
                        transport: Some(TransportRequestFrame {
                            kind: event.frame.kind,
                            stream_id: event.frame.stream_id,
                            index: event.frame.index,
                            duplicate: event.duplicate,
                        }),
                    });
                };
                let request = protocol::parse_request(&payload)?;
                Ok(WriteEvent::Request {
                    request,
                    transport: Some(TransportContext {
                        stream_id: event.frame.stream_id,
                        final_frame_index: event.frame.index,
                    }),
                })
            }
            kind if kind.is_ack() => Ok(WriteEvent::TransportAck(TransportAck {
                kind,
                stream_id: event.frame.stream_id,
                index: event.frame.index,
            })),
            _ => Ok(WriteEvent::Incomplete { transport: None }),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WriteRouterError {
    #[error(transparent)]
    Protocol(#[from] protocol::ProtocolError),
    #[error(transparent)]
    Transport(#[from] protocol::transport::TransportError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_transport_write_is_incomplete_not_bad_json() {
        let request = protocol::CommandRequest::new(
            "req-transport",
            protocol::requests::CommandPayload::SystemStatus,
        );
        let payload = protocol::encode_request(&request).unwrap();
        let frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::RequestChunk,
            3,
            &payload,
            20,
        )
        .unwrap();
        let mut router = WriteRouter::new();

        let event = router.accept_write(&frames[0]).unwrap();

        assert_eq!(
            event,
            WriteEvent::Incomplete {
                transport: Some(TransportRequestFrame {
                    kind: protocol::transport::FrameKind::RequestChunk,
                    stream_id: 3,
                    index: 1,
                    duplicate: false,
                }),
            }
        );
    }

    #[test]
    fn complete_transport_request_is_parsed_after_reassembly() {
        let request = protocol::CommandRequest::new(
            "req-transport",
            protocol::requests::CommandPayload::SystemStatus,
        );
        let payload = protocol::encode_request(&request).unwrap();
        let frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::RequestChunk,
            4,
            &payload,
            20,
        )
        .unwrap();
        let mut router = WriteRouter::new();
        let mut completed = None;
        let frame_count = frames.len() as u8;

        for frame in frames {
            if let WriteEvent::Request { request, transport } = router.accept_write(&frame).unwrap()
            {
                completed = Some((request, transport));
            }
        }

        let (decoded, transport) = completed.expect("request should complete");
        assert_eq!(decoded.id, "req-transport");
        assert_eq!(
            decoded.payload,
            protocol::requests::CommandPayload::SystemStatus
        );
        assert_eq!(
            transport,
            Some(TransportContext {
                stream_id: 4,
                final_frame_index: frame_count,
            })
        );
    }

    #[test]
    fn reset_clears_stale_inbound_transport_streams() {
        let stale_request = protocol::CommandRequest::new(
            "req-stale",
            protocol::requests::CommandPayload::SystemStatus,
        );
        let stale_payload = protocol::encode_request(&stale_request).unwrap();
        let stale_frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::RequestChunk,
            1,
            &stale_payload,
            20,
        )
        .unwrap();
        let fresh_payload = br#"{"id":"a1","cmd":"system.status"}"#;
        let fresh_frames = protocol::transport::encode_payload_frames(
            protocol::transport::FrameKind::RequestChunk,
            1,
            fresh_payload,
            20,
        )
        .unwrap();
        let mut router = WriteRouter::new();

        for frame in stale_frames.iter().take(2) {
            assert!(matches!(
                router.accept_write(frame).unwrap(),
                WriteEvent::Incomplete { .. }
            ));
        }
        router.reset();

        let mut completed = None;
        for frame in fresh_frames {
            if let WriteEvent::Request { request, .. } = router.accept_write(&frame).unwrap() {
                completed = Some(request);
            }
        }

        assert_eq!(completed.expect("fresh request should complete").id, "a1");
    }

    #[test]
    fn compact_ack_routes_without_json_link_ack() {
        let raw = protocol::transport::encode_ack_frame(
            protocol::transport::FrameKind::AckRange,
            9,
            2,
            20,
        )
        .unwrap();
        let mut router = WriteRouter::new();

        let event = router.accept_write(&raw).unwrap();

        assert_eq!(
            event,
            WriteEvent::TransportAck(TransportAck {
                kind: protocol::transport::FrameKind::AckRange,
                stream_id: 9,
                index: 2,
            })
        );
    }

    #[test]
    fn legacy_json_request_still_routes_during_rollout() {
        let request = protocol::CommandRequest::new(
            "req-legacy",
            protocol::requests::CommandPayload::SystemStatus,
        );
        let payload = protocol::encode_request(&request).unwrap();
        let mut router = WriteRouter::new();

        let event = router.accept_write(&payload).unwrap();

        match event {
            WriteEvent::Request { request, transport } => {
                assert_eq!(request.id, "req-legacy");
                assert!(transport.is_none());
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
