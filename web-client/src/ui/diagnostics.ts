import type { CommandResponse, JsonObject, JsonValue } from "../types";
import type { TFunction } from "../i18n/I18nProvider";

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

export function statusView(response: CommandResponse | undefined, t: TFunction): StatusView | undefined {
  const data = response?.data;
  if (!data) return undefined;
  const interfaces = Array.isArray(data.interfaces)
    ? data.interfaces.map(readInterface).filter((item): item is StatusInterfaceView => Boolean(item))
    : [];
  return {
    deviceName: readString(data.device_name) || t("diagnostics.unknownDevice"),
    hostname: readString(data.hostname) || t("diagnostics.unknownHost"),
    system: readString(data.system) || t("diagnostics.unknownSystem"),
    user: readString(data.user) || t("diagnostics.unknownUser"),
    network: readString(data.network) || t("diagnostics.noWifi"),
    ip: readString(data.ip) || interfaces[0]?.ipv4 || t("diagnostics.noIp"),
    interfaces,
  };
}

export function capabilitiesView(response: CommandResponse | undefined, t: TFunction): CapabilitiesView | undefined {
  const data = response?.data;
  if (!data) return undefined;
  return {
    protocolVersion: readString(data.protocol_version) || response?.v || t("diagnostics.unknown"),
    payloadLimit: typeof data.payload_limit === "number" ? `${data.payload_limit} bytes` : t("diagnostics.unknown"),
    commands: readStringList(data.commands),
    features: readStringList(data.features),
  };
}

export function heartbeatAlive(response: CommandResponse | undefined, t: TFunction): string {
  if (!response?.data) return t("diagnostics.heartbeatUnchecked");
  return response.data.alive === true ? t("diagnostics.heartbeatAlive") : t("diagnostics.heartbeatUnconfirmed");
}

export function preferredIp(response?: CommandResponse): string | undefined {
  const ip = typeof response?.data?.ip === "string" ? response.data.ip : undefined;
  if (ip) return ip;
  const interfaces = response?.data?.interfaces;
  if (!Array.isArray(interfaces)) return undefined;
  const first = interfaces.map(readInterface).find(Boolean);
  return first?.ipv4;
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
