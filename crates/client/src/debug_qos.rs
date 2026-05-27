use anyhow::{anyhow, Result};
use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;

use crate::debug_ble::DebugLog;

pub(crate) async fn write_with_qos(
    peripheral: &Peripheral,
    write_char: &Characteristic,
    bytes: &[u8],
    label: &str,
    trace_qos: bool,
    log: &mut DebugLog,
) -> Result<()> {
    if trace_qos {
        log.line(format!(
            "[QOS:tx] {label} bytes={} write=with-response",
            bytes.len()
        ));
    }
    match peripheral
        .write(write_char, bytes, WriteType::WithResponse)
        .await
    {
        Ok(()) => Ok(()),
        Err(err) => {
            if trace_qos {
                log.line(format!(
                    "[QOS:tx] {label} fallback=without-response reason={err}"
                ));
            }
            peripheral
                .write(write_char, bytes, WriteType::WithoutResponse)
                .await
                .map_err(Into::into)
        }
    }
}

pub(crate) async fn send_debug_chunk_ack(
    peripheral: &Peripheral,
    write_char: &Characteristic,
    receipt: &client::response::ChunkReceipt,
    trace_qos: bool,
    log: &mut DebugLog,
) -> Result<()> {
    let request = link_ack_request(
        &receipt.response_id,
        protocol::requests::AckType::Chunk,
        receipt.response_seq,
        Some(receipt.chunk_index),
    );
    let bytes = protocol::encode_request(&request).map_err(|err| anyhow!(err.to_string()))?;
    write_with_qos(peripheral, write_char, &bytes, "chunk-ack", trace_qos, log).await?;
    if trace_qos {
        log.line(format!(
            "[QOS:ack] id={} seq={} chunk={}/{}",
            receipt.response_id, receipt.response_seq, receipt.chunk_index, receipt.chunk_total
        ));
    }
    Ok(())
}

pub(crate) async fn send_debug_event_ack(
    peripheral: &Peripheral,
    write_char: &Characteristic,
    response: &protocol::CommandResponse,
    trace_qos: bool,
    log: &mut DebugLog,
) -> Result<()> {
    let request = link_ack_request(
        &response.id,
        protocol::requests::AckType::Event,
        response.seq,
        None,
    );
    let bytes = protocol::encode_request(&request).map_err(|err| anyhow!(err.to_string()))?;
    write_with_qos(peripheral, write_char, &bytes, "event-ack", trace_qos, log).await?;
    if trace_qos {
        log.line(format!(
            "[QOS:event-ack] id={} seq={}",
            response.id, response.seq
        ));
    }
    Ok(())
}

fn link_ack_request(
    request_id: &str,
    ack_type: protocol::requests::AckType,
    response_seq: u64,
    chunk_index: Option<usize>,
) -> protocol::CommandRequest {
    protocol::CommandRequest::new(
        request_id.to_string(),
        protocol::requests::CommandPayload::LinkAck(protocol::requests::LinkAckArgs {
            ack_type,
            response_seq,
            chunk_index,
        }),
    )
}
