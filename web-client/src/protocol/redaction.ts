import { encodeCommandRequest } from "./commands";
import type { CommandRequest, JsonValue } from "../types";

const SECRET_KEYS = new Set(["pwd", "password", "passphrase", "token", "secret", "key"]);

export function redactSensitiveValue(key: string, value: JsonValue): JsonValue {
  if (SECRET_KEYS.has(key.toLowerCase())) {
    return "***";
  }
  return redactJson(value);
}

export function redactJson(value: JsonValue): JsonValue {
  if (Array.isArray(value)) {
    return value.map((item) => redactJson(item));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, nested]) => [key, redactSensitiveValue(key, nested)]),
    );
  }
  return value;
}

export function traceBytes(request: CommandRequest, redactSecrets: boolean): Uint8Array {
  if (!redactSecrets) {
    return encodeCommandRequest(request);
  }
  return encodeCommandRequest(redactCommandRequest(request));
}

function redactCommandRequest(request: CommandRequest): CommandRequest {
  return {
    ...request,
    args: redactJson(request.args) as CommandRequest["args"],
  };
}
