import type { CommandResponse, DebugMode, TraceEntry } from "../types";

export interface GatewayClientOptions {
  debugMode: DebugMode;
  timeoutMs?: number;
  onTrace?: (entry: TraceEntry) => void;
  onEvent?: (response: CommandResponse) => void;
  onDisconnect?: () => void;
}
