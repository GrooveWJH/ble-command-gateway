import { traceEntry } from "../services/trace";
import type { GatewayCommand, JsonObject, TraceEntry } from "../types";

export function sendRawCommand(
  command: GatewayCommand,
  args: string,
  run: (cmd: GatewayCommand, args?: JsonObject) => void,
  trace: (entry: TraceEntry) => void,
) {
  try {
    run(command, JSON.parse(args) as JsonObject);
  } catch {
    trace(traceEntry("ERR", "Raw args must be valid JSON object", undefined, "error"));
  }
}
