import { encodeCommandRequest } from "../protocol/commands";
import type { CommandRequest } from "../types";
import { formatBytes } from "./trace";

export function bytesDetail(bytes: Uint8Array): string {
  return `${new TextDecoder().decode(bytes)}\n\nhex ${formatBytes(bytes)}`;
}

export function ackDetail(request: CommandRequest): string {
  const bytes = encodeCommandRequest(request);
  return `${JSON.stringify(request)}\n\nhex ${formatBytes(bytes)}`;
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
