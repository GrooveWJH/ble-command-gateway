import { writeWithResponse, type BleUartConnection } from "../ble/webBluetooth";
import { buildLinkAckRequest, encodeCommandRequest } from "../protocol/commands";
import type { ChunkReceipt } from "../protocol/responseDecoder";
import type { CommandResponse, TraceEntry } from "../types";
import { ackDetail } from "./traceDetails";

export type TraceFn = (
  label: string,
  summary: string,
  detail?: string,
  level?: TraceEntry["level"],
) => void;

export async function sendChunkAck(
  connection: BleUartConnection,
  receipt: ChunkReceipt,
  trace: TraceFn,
): Promise<void> {
  const request = buildLinkAckRequest(
    receipt.responseId,
    "chunk",
    receipt.responseSeq,
    receipt.chunkIndex,
  );
  const bytes = encodeCommandRequest(request);
  trace("TX:raw", `kind=chunk-ack bytes=${bytes.length}`, ackDetail(request));
  await writeWithResponse(connection.writeCharacteristic, bytes);
  trace(
    "QOS:ack",
    `id=${receipt.responseId} seq=${receipt.responseSeq} chunk=${receipt.chunkIndex}/${receipt.chunkTotal}`,
  );
}

export async function sendEventAck(
  connection: BleUartConnection,
  response: CommandResponse,
  trace: TraceFn,
): Promise<void> {
  const request = buildLinkAckRequest(response.id, "event", response.seq);
  const bytes = encodeCommandRequest(request);
  trace("TX:raw", `kind=event-ack bytes=${bytes.length}`, ackDetail(request));
  await writeWithResponse(connection.writeCharacteristic, bytes);
  trace("QOS:event-ack", `id=${response.id} seq=${response.seq}`);
}
