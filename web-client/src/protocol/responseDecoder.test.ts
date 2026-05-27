import { describe, expect, it } from "vitest";

import { ResponseDecoder } from "./responseDecoder";

const enc = new TextEncoder();

describe("response_json chunk decoder", () => {
  it("reports chunk ACK receipts and assembles complete response JSON", () => {
    const decoder = new ResponseDecoder();
    const finalResponse = JSON.stringify({
      id: "req-1",
      cmd: "wifi.scan",
      phase: "result",
      seq: 12,
      final: true,
      ok: true,
      code: "OK",
      text: "wifi scan complete",
      data: { count: 1, networks: [{ ssid: "Yundrone", signal: 90, channel: "1" }] },
      v: "YundroneBT-V2.1.0",
    });
    const splitAt = Math.ceil(finalResponse.length / 2);
    const chunk1 = chunk("req-1", 12, 1, 2, finalResponse.slice(0, splitAt));
    const chunk2 = chunk("req-1", 12, 2, 2, finalResponse.slice(splitAt));

    const first = decoder.decode(enc.encode(JSON.stringify(chunk1)));
    const second = decoder.decode(enc.encode(JSON.stringify(chunk2)));

    expect(first.chunkReceipt).toEqual({
      responseId: "req-1",
      responseSeq: 12,
      chunkIndex: 1,
      chunkTotal: 2,
    });
    expect(first.response).toBeUndefined();
    expect(second.response?.text).toBe("wifi scan complete");
    expect(second.assembledFromChunks).toBe(true);
  });

  it("ignores duplicate chunk for assembly count", () => {
    const decoder = new ResponseDecoder();
    const finalResponse = JSON.stringify({
      id: "req-2",
      ok: true,
      code: "OK",
      text: "done",
      v: "YundroneBT-V2.1.0",
    });
    const firstPayload = finalResponse.slice(0, 20);
    const secondPayload = finalResponse.slice(20);
    const first = enc.encode(JSON.stringify(chunk("req-2", 1, 1, 2, firstPayload)));
    const second = enc.encode(JSON.stringify(chunk("req-2", 1, 2, 2, secondPayload)));

    expect(decoder.decode(first).response).toBeUndefined();
    expect(decoder.decode(first).response).toBeUndefined();
    expect(decoder.decode(second).response?.id).toBe("req-2");
  });

  it("keeps chunk sessions separate by response id and seq", () => {
    const decoder = new ResponseDecoder();
    const progress = JSON.stringify({
      id: "req-3",
      cmd: "wifi.scan",
      phase: "progress",
      seq: 2,
      final: false,
      ok: true,
      code: "IN_PROGRESS",
      text: "please wait",
      v: "YundroneBT-V2.1.0",
    });
    const result = JSON.stringify({
      id: "req-3",
      cmd: "wifi.scan",
      phase: "result",
      seq: 3,
      final: true,
      ok: true,
      code: "OK",
      text: "done",
      data: { networks: [] },
      v: "YundroneBT-V2.1.0",
    });

    const progressChunk = chunk("req-3", 2, 1, 2, progress.slice(0, 12));
    const resultChunk = chunk("req-3", 3, 1, 2, result.slice(0, 12));

    decoder.decode(enc.encode(JSON.stringify(progressChunk)));
    decoder.decode(enc.encode(JSON.stringify(resultChunk)));
    const assembledProgress = decoder.decode(
      enc.encode(JSON.stringify(chunk("req-3", 2, 2, 2, progress.slice(12)))),
    );
    const assembledResult = decoder.decode(
      enc.encode(JSON.stringify(chunk("req-3", 3, 2, 2, result.slice(12)))),
    );

    expect(assembledProgress.response).toMatchObject({
      id: "req-3",
      seq: 2,
      phase: "progress",
      final: false,
    });
    expect(assembledResult.response).toMatchObject({
      id: "req-3",
      seq: 3,
      phase: "result",
      final: true,
      text: "done",
    });
  });
});

function chunk(id: string, seq: number, index: number, total: number, payload: string) {
  return {
    id,
    cmd: "wifi.scan",
    phase: "result",
    seq,
    final: true,
    ok: true,
    code: "OK",
    text: "",
    data: {
      chunk: {
        mode: "response_json",
        index,
        total,
        payload,
        ack_required: true,
      },
    },
    v: "YundroneBT-V2.1.0",
  };
}
