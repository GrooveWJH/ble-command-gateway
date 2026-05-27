import { writeWithResponse, type BleUartConnection } from "../ble/webBluetooth";
import { buildCommandRequest, commandLabel, encodeCommandRequest } from "../protocol/commands";
import { ResponseDecoder } from "../protocol/responseDecoder";
import { traceBytes } from "../protocol/redaction";
import type {
  CommandRequest,
  CommandResponse,
  DebugMode,
  GatewayCommand,
  JsonObject,
  TraceEntry,
} from "../types";
import { commandSummary, formatBytes, responseSummary, traceEntry } from "./trace";
import { clearPendingTimers, type PendingCommand } from "./pending";
import { sendChunkAck, sendEventAck } from "./ack";
import { bytesDetail, errorMessage } from "./traceDetails";
const REQUEST_ACCEPT_RETRIES = 3;
const REQUEST_ACCEPT_TIMEOUT_MS = 3_000;

export interface GatewayClientOptions {
  debugMode: DebugMode;
  timeoutMs?: number;
  onTrace?: (entry: TraceEntry) => void;
  onEvent?: (response: CommandResponse) => void;
  onDisconnect?: () => void;
}

export class GatewayClient {
  private readonly decoder = new ResponseDecoder();
  private readonly connection: BleUartConnection;
  private readonly onTrace?: (entry: TraceEntry) => void;
  private readonly onEvent?: (response: CommandResponse) => void;
  private readonly onDisconnect?: () => void;
  private readonly timeoutMs: number;
  private debugMode: DebugMode;
  private pending?: PendingCommand;
  private queue: Promise<unknown> = Promise.resolve();

  constructor(connection: BleUartConnection, options: GatewayClientOptions) {
    this.connection = connection;
    this.debugMode = options.debugMode;
    this.timeoutMs = options.timeoutMs ?? 30_000;
    this.onTrace = options.onTrace;
    this.onEvent = options.onEvent;
    this.onDisconnect = options.onDisconnect;
    this.connection.notifyCharacteristic.addEventListener(
      "characteristicvaluechanged",
      this.handleNotification,
    );
    this.connection.device.addEventListener("gattserverdisconnected", this.handleDisconnect);
  }

  setDebugMode(debugMode: DebugMode): void {
    this.debugMode = debugMode;
  }

  runCommand(cmd: GatewayCommand, args: JsonObject = {}): Promise<CommandResponse> {
    const execution = this.queue
      .catch(() => undefined)
      .then(() => this.executeCommand(cmd, args));
    this.queue = execution;
    return execution;
  }

  disconnect(): void {
    this.connection.notifyCharacteristic.removeEventListener(
      "characteristicvaluechanged",
      this.handleNotification,
    );
    this.connection.device.removeEventListener("gattserverdisconnected", this.handleDisconnect);
    this.failPending(new Error("Device disconnected"));
    this.connection.disconnect();
  }

  private async executeCommand(cmd: GatewayCommand, args: JsonObject): Promise<CommandResponse> {
    if (cmd === "link.ack") {
      throw new Error("link.ack is transport-only and cannot be sent as a user command");
    }
    const request = buildCommandRequest(cmd, args);
    const bytes = encodeCommandRequest(request);
    this.traceTxRaw(request);
    this.trace("QOS:tx", `kind=request bytes=${bytes.length} write=with-response`);
    return new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        if (this.pending?.request.id === request.id) {
          clearPendingTimers(this.pending);
          this.pending = undefined;
        }
        reject(new Error(`Timed out waiting for ${commandLabel(cmd)} after ${this.timeoutMs}ms`));
      }, this.timeoutMs);
      this.pending = {
        request,
        requestBytes: bytes,
        resolve,
        reject,
        timer,
        accepted: false,
        attempts: 0,
        events: [],
      };
      this.trace("CMD:sent", commandSummary(cmd, request.id));
      this.writePendingRequest(request.id);
    });
  }

  private writePendingRequest(requestId: string): void {
    const pending = this.pending;
    if (!pending || pending.request.id !== requestId || pending.accepted) {
      return;
    }
    pending.attempts += 1;
    const attempt = pending.attempts;
    void writeWithResponse(this.connection.writeCharacteristic, pending.requestBytes)
      .then(() => {
        const latest = this.pending;
        if (!latest || latest.request.id !== requestId || latest.accepted) {
          return;
        }
        if (attempt >= REQUEST_ACCEPT_RETRIES) {
          this.failPending(new Error("request was not accepted"));
          return;
        }
        latest.acceptTimer = window.setTimeout(() => {
          if (!this.pending || this.pending.request.id !== requestId || this.pending.accepted) {
            return;
          }
          this.trace(
            "QOS:retry",
            `id=${requestId} attempt=${attempt + 1}/${REQUEST_ACCEPT_RETRIES}`,
            undefined,
            "warn",
          );
          this.writePendingRequest(requestId);
        }, REQUEST_ACCEPT_TIMEOUT_MS);
      })
      .catch((error) => {
        if (this.pending?.request.id === requestId) {
          this.failPending(error instanceof Error ? error : new Error(String(error)));
        }
      });
  }

  private readonly handleNotification = (event: Event) => {
    const characteristic = event.target as BluetoothRemoteGATTCharacteristic;
    const value = characteristic.value;
    if (!value) {
      return;
    }
    const raw = new Uint8Array(value.buffer.slice(value.byteOffset, value.byteOffset + value.byteLength));
    this.trace("RX:raw", `bytes=${raw.length}`, bytesDetail(raw));
    let decoded;
    try {
      decoded = this.decoder.decode(raw);
    } catch (error) {
      this.trace("RX:error", error instanceof Error ? error.message : String(error), undefined, "error");
      return;
    }

    if (decoded.chunkReceipt) {
      void sendChunkAck(this.connection, decoded.chunkReceipt, this.trace).catch((error) => {
        this.trace("QOS:ack-error", errorMessage(error), undefined, "warn");
      });
      this.trace(
        "RX:chunk",
        `id=${decoded.chunkReceipt.responseId} index=${decoded.chunkReceipt.chunkIndex}/${decoded.chunkReceipt.chunkTotal}`,
      );
    }

    if (!decoded.response) {
      return;
    }

    const response = decoded.response;
    if (decoded.assembledFromChunks) {
      this.trace(
        "RX:assembled",
        `bytes=${new TextEncoder().encode(JSON.stringify(response)).length}`,
        JSON.stringify(response, null, 2),
      );
    }
    void sendEventAck(this.connection, response, this.trace).catch((error) => {
      this.trace("QOS:event-ack-error", errorMessage(error), undefined, "warn");
    });
    this.trace("RX:frame", responseSummary(response), JSON.stringify(response, null, 2));
    this.onEvent?.(response);
    this.resolveIfFinal(response);
  };

  private resolveIfFinal(response: CommandResponse): void {
    if (!this.pending || response.id !== this.pending.request.id) {
      return;
    }
    if (!this.pending.accepted) {
      this.pending.accepted = true;
      if (this.pending.acceptTimer !== undefined) {
        window.clearTimeout(this.pending.acceptTimer);
        this.pending.acceptTimer = undefined;
      }
    }
    this.pending.events.push(response);
    if (!response.final) {
      return;
    }
    clearPendingTimers(this.pending);
    const pending = this.pending;
    this.pending = undefined;
    pending.resolve(response);
  }

  private readonly handleDisconnect = () => {
    this.failPending(new Error("Device disconnected"));
    this.trace("SYS", "Device disconnected", undefined, "warn");
    this.onDisconnect?.();
  };

  private failPending(error: Error): void {
    if (!this.pending) {
      return;
    }
    clearPendingTimers(this.pending);
    const pending = this.pending;
    this.pending = undefined;
    pending.reject(error);
  }

  private traceTxRaw(request: CommandRequest): void {
    if (this.debugMode === "off") {
      return;
    }
    const redacted = this.debugMode === "safe";
    const bytes = traceBytes(request, redacted);
    this.trace(
      "TX:raw",
      `kind=request bytes=${bytes.length} redacted=${redacted}`,
      `${new TextDecoder().decode(bytes)}\n\nhex ${formatBytes(bytes)}`,
    );
  }

  private readonly trace = (
    label: string,
    summary: string,
    detail?: string,
    level: TraceEntry["level"] = "info",
  ): void => {
    if (this.debugMode === "off" && !label.startsWith("CMD")) {
      return;
    }
    this.onTrace?.(traceEntry(label, summary, detail, level));
  };
}
