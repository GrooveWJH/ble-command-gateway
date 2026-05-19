use anyhow::{anyhow, Result};
use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use tracing::info;

pub(crate) const REQUEST_ACCEPT_RETRIES: usize = 3;
pub(crate) const REQUEST_ACCEPT_TIMEOUT_SECS: u64 = 3;

pub(crate) async fn write_payload(
    device: &Peripheral,
    write_char: &Characteristic,
    device_name: &str,
    device_rssi: Option<i16>,
    payload: &[u8],
) -> Result<()> {
    if let Err(err) = device
        .write(write_char, payload, WriteType::WithResponse)
        .await
    {
        info!(
            device_name,
            rssi = ?device_rssi,
            error = %err,
            "qos.write.fallback"
        );
        device
            .write(write_char, payload, WriteType::WithoutResponse)
            .await?;
    }
    Ok(())
}

pub(crate) async fn send_chunk_ack(
    device: &Peripheral,
    write_char: &Characteristic,
    device_name: &str,
    device_rssi: Option<i16>,
    receipt: &crate::response::ChunkReceipt,
) -> Result<()> {
    send_link_ack(
        device,
        write_char,
        device_name,
        device_rssi,
        &receipt.response_id,
        protocol::requests::LinkAckArgs {
            ack_type: protocol::requests::AckType::Chunk,
            response_seq: receipt.response_seq,
            chunk_index: Some(receipt.chunk_index),
        },
    )
    .await?;
    info!(
        device_name,
        rssi = ?device_rssi,
        response_id = %receipt.response_id,
        response_seq = receipt.response_seq,
        chunk_index = receipt.chunk_index,
        chunk_total = receipt.chunk_total,
        "qos.chunk.ack.sent"
    );
    Ok(())
}

pub(crate) async fn send_event_ack(
    device: &Peripheral,
    write_char: &Characteristic,
    device_name: &str,
    device_rssi: Option<i16>,
    response: &protocol::CommandResponse,
) -> Result<()> {
    send_link_ack(
        device,
        write_char,
        device_name,
        device_rssi,
        &response.id,
        protocol::requests::LinkAckArgs {
            ack_type: protocol::requests::AckType::Event,
            response_seq: response.seq,
            chunk_index: None,
        },
    )
    .await?;
    info!(
        device_name,
        rssi = ?device_rssi,
        response_id = %response.id,
        response_seq = response.seq,
        "qos.event.ack.sent"
    );
    Ok(())
}

async fn send_link_ack(
    device: &Peripheral,
    write_char: &Characteristic,
    device_name: &str,
    device_rssi: Option<i16>,
    request_id: &str,
    ack: protocol::requests::LinkAckArgs,
) -> Result<()> {
    let request = protocol::CommandRequest::new(
        request_id.to_string(),
        protocol::requests::CommandPayload::LinkAck(ack),
    );
    let payload = protocol::encode_request(&request).map_err(|err| anyhow!(err.to_string()))?;
    write_payload(device, write_char, device_name, device_rssi, &payload).await
}
