pub struct ResponseDecoder {
    assembler: protocol::chunking::ChunkAssembler,
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
        let response = protocol::parse_response(raw)?;
        self.assembler.add_chunk(response)
    }
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
}
