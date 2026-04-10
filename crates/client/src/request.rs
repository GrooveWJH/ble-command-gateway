use anyhow::anyhow;

#[derive(Debug, Clone)]
pub struct PreparedRequest {
    pub request: protocol::CommandRequest,
    pub bytes: Vec<u8>,
}

pub fn build_request(payload: protocol::requests::CommandPayload) -> protocol::CommandRequest {
    protocol::CommandRequest::new(uuid::Uuid::new_v4().to_string(), payload)
}

pub fn prepare_request(
    payload: protocol::requests::CommandPayload,
) -> anyhow::Result<PreparedRequest> {
    let request = build_request(payload);
    let bytes = protocol::encode_request(&request).map_err(|err| anyhow!(err.to_string()))?;
    Ok(PreparedRequest { request, bytes })
}

pub fn encode_request_bytes(
    payload: protocol::requests::CommandPayload,
) -> anyhow::Result<Vec<u8>> {
    prepare_request(payload).map(|prepared| prepared.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_populates_id_and_defaults() {
        let request = build_request(protocol::requests::CommandPayload::Status);

        assert!(!request.id.is_empty());
        assert_eq!(request.payload, protocol::requests::CommandPayload::Status);
        assert_eq!(request.v, protocol::PROTOCOL_VERSION);
    }

    #[test]
    fn encode_request_bytes_produces_protocol_schema() {
        let payload =
            encode_request_bytes(protocol::requests::CommandPayload::WifiScan { ifname: None })
                .unwrap();
        let request = protocol::parse_request(&payload).unwrap();

        assert!(!request.id.is_empty());
        assert_eq!(
            request.payload,
            protocol::requests::CommandPayload::WifiScan { ifname: None }
        );
    }

    #[test]
    fn prepare_request_keeps_request_id_inside_encoded_bytes() {
        let prepared = prepare_request(protocol::requests::CommandPayload::Ping).unwrap();
        let decoded = protocol::parse_request(&prepared.bytes).unwrap();

        assert_eq!(decoded.id, prepared.request.id);
        assert_eq!(decoded.payload, protocol::requests::CommandPayload::Ping);
    }
}
