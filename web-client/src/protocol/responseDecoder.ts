import { parseCommandResponse } from "./commands";
import type { CommandResponse, JsonObject } from "../types";

export interface ChunkReceipt {
  responseId: string;
  responseSeq: number;
  chunkIndex: number;
  chunkTotal: number;
}

export interface DecodedEvent {
  chunkReceipt?: ChunkReceipt;
  response?: CommandResponse;
  assembledFromChunks: boolean;
}

interface ResponseJsonChunk {
  mode: "response_json";
  index: number;
  total: number;
  payload: string;
  ack_required?: boolean;
}

interface ChunkState {
  total: number;
  received: number;
  parts: string[];
}

export class ResponseDecoder {
  private readonly sessions = new Map<string, ChunkState>();

  decode(raw: Uint8Array): DecodedEvent {
    const response = parseCommandResponse(raw);
    const chunk = readResponseJsonChunk(response.data);
    if (!chunk) {
      return { response, assembledFromChunks: false };
    }

    const receipt = chunk.ack_required
      ? {
          responseId: response.id,
          responseSeq: response.seq,
          chunkIndex: chunk.index,
          chunkTotal: chunk.total,
        }
      : undefined;
    const assembled = this.addChunk(chunkSessionKey(response.id, response.seq), chunk);
    return {
      chunkReceipt: receipt,
      response: assembled ? parseCommandResponse(new TextEncoder().encode(assembled)) : undefined,
      assembledFromChunks: true,
    };
  }

  private addChunk(sessionKey: string, chunk: ResponseJsonChunk): string | undefined {
    const state =
      this.sessions.get(sessionKey) ??
      {
        total: chunk.total,
        received: 0,
        parts: Array.from({ length: chunk.total }, () => ""),
      };
    this.sessions.set(sessionKey, state);

    if (chunk.index >= 1 && chunk.index <= state.total) {
      const slot = chunk.index - 1;
      if (!state.parts[slot] && chunk.payload) {
        state.received += 1;
      }
      state.parts[slot] = chunk.payload;
    }

    if (state.received === state.total) {
      this.sessions.delete(sessionKey);
      return state.parts.join("");
    }
    return undefined;
  }
}

function chunkSessionKey(responseId: string, responseSeq: number): string {
  return `${responseId}:${responseSeq}`;
}

function readResponseJsonChunk(data?: JsonObject): ResponseJsonChunk | undefined {
  const chunk = data?.chunk;
  if (!chunk || typeof chunk !== "object" || Array.isArray(chunk)) {
    return undefined;
  }
  const object = chunk as JsonObject;
  if (
    object.mode !== "response_json" ||
    typeof object.index !== "number" ||
    typeof object.total !== "number" ||
    typeof object.payload !== "string"
  ) {
    return undefined;
  }
  return {
    mode: "response_json",
    index: object.index,
    total: object.total,
    payload: object.payload,
    ack_required: object.ack_required === true,
  };
}
