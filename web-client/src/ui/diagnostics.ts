import type { CommandResponse, JsonObject, JsonValue } from "../types";

export interface StatusInterfaceView {
  ifname: string;
  kind: string;
  ipv4: string;
}

export interface StatusView {
  deviceName: string;
  hostname: string;
  system: string;
  user: string;
  network: string;
  ip: string;
  interfaces: StatusInterfaceView[];
}

export interface CapabilitiesView {
  protocolVersion: string;
  payloadLimit: string;
  commands: string[];
  features: string[];
}

export function statusView(response?: CommandResponse): StatusView | undefined {
  const data = response?.data;
  if (!data) return undefined;
  const interfaces = Array.isArray(data.interfaces)
    ? data.interfaces.map(readInterface).filter((item): item is StatusInterfaceView => Boolean(item))
    : [];
  return {
    deviceName: readString(data.device_name) || "未知设备名",
    hostname: readString(data.hostname) || "未知主机",
    system: readString(data.system) || "未知系统",
    user: readString(data.user) || "未知用户",
    network: readString(data.network) || "未连接 Wi-Fi",
    ip: readString(data.ip) || interfaces[0]?.ipv4 || "未获取",
    interfaces,
  };
}

export function capabilitiesView(response?: CommandResponse): CapabilitiesView | undefined {
  const data = response?.data;
  if (!data) return undefined;
  return {
    protocolVersion: readString(data.protocol_version) || response?.v || "未知",
    payloadLimit: typeof data.payload_limit === "number" ? `${data.payload_limit} bytes` : "未知",
    commands: readStringList(data.commands),
    features: readStringList(data.features),
  };
}

export function heartbeatAlive(response?: CommandResponse): string {
  if (!response?.data) return "尚未检测";
  return response.data.alive === true ? "alive" : "未确认";
}

export function preferredIp(response?: CommandResponse): string | undefined {
  const view = statusView(response);
  return view?.ip && view.ip !== "未获取" ? view.ip : undefined;
}

function readInterface(value: JsonValue): StatusInterfaceView | undefined {
  if (!isObject(value)) return undefined;
  const ifname = readString(value.ifname);
  const ipv4 = readString(value.ipv4);
  if (!ifname || !ipv4) return undefined;
  return {
    ifname,
    ipv4,
    kind: readString(value.kind) || "other",
  };
}

function readString(value: JsonValue | undefined): string {
  return typeof value === "string" ? value : "";
}

function readStringList(value: JsonValue | undefined): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

function isObject(value: JsonValue): value is JsonObject {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}
