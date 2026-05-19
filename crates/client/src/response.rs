pub struct ResponseDecoder {
    assembler: protocol::chunking::ChunkAssembler,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkReceipt {
    pub response_id: String,
    pub response_seq: u64,
    pub chunk_index: usize,
    pub chunk_total: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedEvent {
    pub chunk_receipt: Option<ChunkReceipt>,
    pub response: Option<protocol::CommandResponse>,
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
        let response = self.assembler.add_chunk(response)?;
        Ok(DecodedEvent {
            chunk_receipt,
            response,
        })
    }
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
