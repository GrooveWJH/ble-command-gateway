import { describe, expect, it } from "vitest";

import {
  PROTOCOL_VERSION,
  buildCommandRequest,
  buildLinkAckRequest,
  commandLabel,
  encodeCommandRequest,
  parseCommandResponse,
} from "./commands";

describe("main branch JSON command protocol", () => {
  it.each([
    ["link.heartbeat", {}, {}],
    ["system.status", {}, {}],
    ["system.capabilities", {}, {}],
    ["wifi.scan", { ifname: null }, {}],
    ["wifi.provision", { ssid: "Yundrone", pwd: "secret" }, { ssid: "Yundrone", pwd: "secret" }],
    ["wifi.profiles.list", {}, {}],
    ["wifi.profiles.delete", { uuids: ["abc"], force: true }, { uuids: ["abc"], force: true }],
  ] as const)("builds %s requests compatible with Rust protocol", (cmd, args, expectedArgs) => {
    const request = buildCommandRequest(cmd, args, "fixed-id");
    const wire = JSON.parse(new TextDecoder().decode(encodeCommandRequest(request)));

    expect(wire).toEqual({
      id: "fixed-id",
      cmd,
      args: expectedArgs,
      v: PROTOCOL_VERSION,
    });
    expect(commandLabel(cmd).length).toBeGreaterThan(0);
  });

  it("builds link.ack requests for chunk and event delivery", () => {
    expect(buildLinkAckRequest("resp-1", "chunk", 4, 2)).toMatchObject({
      id: "resp-1",
      cmd: "link.ack",
      args: { ack_type: "chunk", response_seq: 4, chunk_index: 2 },
    });
    expect(buildLinkAckRequest("resp-1", "event", 4)).toMatchObject({
      id: "resp-1",
      cmd: "link.ack",
      args: { ack_type: "event", response_seq: 4 },
    });
  });

  it("parses response defaults like Rust protocol", () => {
    const response = parseCommandResponse(
      new TextEncoder().encode(
        JSON.stringify({
          id: "fixed-id",
          ok: true,
          code: "OK",
          text: "ready",
          v: PROTOCOL_VERSION,
        }),
      ),
    );

    expect(response.phase).toBe("result");
    expect(response.seq).toBe(1);
    expect(response.final).toBe(true);
  });
});
