import { commandProtocolLabel } from "../protocol/commands";
import type { CommandResponse, GatewayCommand, TraceEntry } from "../types";

let traceSequence = 0;

export function traceEntry(
  label: string,
  summary: string,
  detail?: string,
  level: TraceEntry["level"] = "info",
): TraceEntry {
  traceSequence += 1;
  return {
    id: `${Date.now().toString(36)}-${traceSequence}`,
    at: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
    level,
    label,
    summary,
    detail,
  };
}

export function formatBytes(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((byte) => byte.toString(16).toUpperCase().padStart(2, "0"))
    .join(" ");
}

export function responseSummary(response: CommandResponse): string {
  const cmd = response.cmd ? commandProtocolLabel(response.cmd) : "response";
  return `${cmd} ${response.code} phase=${response.phase} final=${response.final} seq=${response.seq}`;
}

export function commandSummary(cmd: GatewayCommand, requestId: string): string {
  return `${commandProtocolLabel(cmd)} id=${requestId}`;
}
