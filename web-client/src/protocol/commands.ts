import type { CommandRequest, CommandResponse, GatewayCommand, JsonObject } from "../types";

export const PROTOCOL_VERSION = "YundroneBT-V2.1.0";
export const USER_COMMANDS = [
  "link.heartbeat",
  "system.status",
  "system.capabilities",
  "wifi.scan",
  "wifi.provision",
  "wifi.profiles.list",
  "wifi.profiles.delete",
] as const satisfies readonly GatewayCommand[];

const EMPTY_ARGS = new Set<GatewayCommand>([
  "link.heartbeat",
  "system.status",
  "system.capabilities",
  "wifi.profiles.list",
]);

export function createRequestId(): string {
  if (globalThis.crypto?.randomUUID) {
    return globalThis.crypto.randomUUID();
  }
  return `web-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

export function buildCommandRequest(
  cmd: GatewayCommand,
  args: JsonObject = {},
  id = createRequestId(),
): CommandRequest {
  return {
    id,
    cmd,
    args: normalizeArgs(cmd, args),
    v: PROTOCOL_VERSION,
  };
}

export function encodeCommandRequest(request: CommandRequest): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(request));
}

export function parseCommandResponse(bytes: Uint8Array): CommandResponse {
  const value = JSON.parse(new TextDecoder().decode(bytes)) as Partial<CommandResponse>;
  if (
    !value.id ||
    typeof value.ok !== "boolean" ||
    typeof value.code !== "string" ||
    typeof value.text !== "string"
  ) {
    throw new Error("invalid command response");
  }
  return {
    id: value.id,
    cmd: value.cmd,
    phase: value.phase ?? "result",
    seq: value.seq ?? 1,
    final: value.final ?? true,
    ok: value.ok,
    code: value.code,
    text: value.text,
    data: value.data,
    v: value.v ?? PROTOCOL_VERSION,
  };
}

export function buildLinkAckRequest(
  responseId: string,
  ackType: "chunk" | "event",
  responseSeq: number,
  chunkIndex?: number,
): CommandRequest {
  const args: JsonObject = {
    ack_type: ackType,
    response_seq: responseSeq,
  };
  if (chunkIndex !== undefined) {
    args.chunk_index = chunkIndex;
  }
  return buildCommandRequest("link.ack", args, responseId);
}

export function commandLabel(cmd: GatewayCommand): string {
  switch (cmd) {
    case "link.heartbeat":
      return "链路心跳";
    case "system.status":
      return "系统及网络状态";
    case "system.capabilities":
      return "协议能力";
    case "wifi.scan":
      return "扫描周边 Wi-Fi";
    case "wifi.provision":
      return "下发 Wi-Fi 配网";
    case "wifi.profiles.list":
      return "读取已保存 Wi-Fi";
    case "wifi.profiles.delete":
      return "删除 Wi-Fi profile";
    case "link.ack":
      return "链路 ACK";
  }
}

function normalizeArgs(cmd: GatewayCommand, args: JsonObject): JsonObject {
  if (EMPTY_ARGS.has(cmd)) {
    return {};
  }
  if (cmd === "wifi.scan") {
    return typeof args.ifname === "string" && args.ifname.length > 0
      ? { ifname: args.ifname }
      : {};
  }
  if (cmd === "wifi.provision") {
    const ssid = typeof args.ssid === "string" ? args.ssid.trim() : "";
    if (!ssid) {
      throw new Error("wifi.provision requires ssid");
    }
    const normalized: JsonObject = { ssid };
    if (typeof args.pwd === "string" && args.pwd.length > 0) {
      normalized.pwd = args.pwd;
    }
    return normalized;
  }
  if (cmd === "wifi.profiles.delete") {
    const uuids = Array.isArray(args.uuids)
      ? args.uuids.filter((uuid): uuid is string => typeof uuid === "string" && uuid.length > 0)
      : [];
    if (uuids.length === 0) {
      throw new Error("wifi.profiles.delete requires at least one uuid");
    }
    return { uuids, force: args.force === true };
  }
  return args;
}
