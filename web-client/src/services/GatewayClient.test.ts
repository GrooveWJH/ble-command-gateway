import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GatewayClient } from "./GatewayClient";
import type { BleUartConnection } from "../ble/webBluetooth";

describe("GatewayClient reliability", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("registers the pending request before writing so fast responses are not lost", async () => {
    const connection = createConnection();
    const client = new GatewayClient(connection, { debugMode: "off" });
    const responsePromise = client.runCommand("system.status");

    await flushMicrotasks();
    expect(connection.writes).toHaveLength(1);
    const request = JSON.parse(new TextDecoder().decode(connection.writes[0] as Uint8Array));
    connection.emitNotify({
      id: request.id,
      cmd: "system.status",
      ok: true,
      code: "OK",
      text: "ready",
      v: "YundroneBT-V2.1.0",
    });

    await expect(responsePromise).resolves.toMatchObject({
      id: request.id,
      code: "OK",
      text: "ready",
    });
    expect(connection.writes.length).toBeGreaterThanOrEqual(2);
    const ack = JSON.parse(new TextDecoder().decode(connection.writes.at(-1) as Uint8Array));
    expect(ack).toMatchObject({
      id: request.id,
      cmd: "link.ack",
      args: { ack_type: "event", response_seq: 1 },
    });
  });

  it("rejects an in-flight command when the BLE device disconnects", async () => {
    const connection = createConnection();
    const client = new GatewayClient(connection, { debugMode: "off" });
    const responsePromise = client.runCommand("system.status");

    await flushMicrotasks();
    expect(connection.writes).toHaveLength(1);
    connection.device.dispatchEvent(new Event("gattserverdisconnected"));

    await expect(responsePromise).rejects.toThrow("Device disconnected");
  });

  it("retries the same request id when no acceptance event arrives", async () => {
    const connection = createConnection();
    const client = new GatewayClient(connection, { debugMode: "off", timeoutMs: 10_000 });
    const responsePromise = client.runCommand("wifi.scan");

    await flushMicrotasks();
    expect(connection.writes).toHaveLength(1);
    const firstRequest = JSON.parse(new TextDecoder().decode(connection.writes[0] as Uint8Array));
    await vi.advanceTimersByTimeAsync(3_000);
    await flushMicrotasks();
    expect(connection.writes).toHaveLength(2);
    const secondRequest = JSON.parse(new TextDecoder().decode(connection.writes[1] as Uint8Array));

    expect(secondRequest).toMatchObject({
      id: firstRequest.id,
      cmd: "wifi.scan",
    });

    connection.emitNotify({
      id: firstRequest.id,
      cmd: "wifi.scan",
      phase: "result",
      seq: 1,
      final: true,
      ok: true,
      code: "OK",
      text: "wifi scan complete",
      data: { networks: [] },
      v: "YundroneBT-V2.1.0",
    });

    await expect(responsePromise).resolves.toMatchObject({
      id: firstRequest.id,
      code: "OK",
    });
  });

  it("fails quickly when a request is never accepted after retries", async () => {
    const connection = createConnection();
    const client = new GatewayClient(connection, { debugMode: "off", timeoutMs: 30_000 });
    const responsePromise = client.runCommand("wifi.scan");
    const rejection = expect(responsePromise).rejects.toThrow("request was not accepted");

    await flushMicrotasks();
    await vi.advanceTimersByTimeAsync(9_000);
    await flushMicrotasks();

    expect(connection.writes).toHaveLength(3);
    await rejection;
  });
});

function createConnection() {
  const notifyCharacteristic = new EventTarget() as BluetoothRemoteGATTCharacteristic;
  const writeCharacteristic = {
    writeValueWithResponse: vi.fn(async (value: BufferSource) => {
      connection.writes.push(new Uint8Array(value as ArrayBuffer));
    }),
  } as unknown as BluetoothRemoteGATTCharacteristic;
  const device = new EventTarget() as BluetoothDevice;
  Object.assign(device, { name: "yundrone-abc123" });

  const connection = {
    device,
    server: {} as BluetoothRemoteGATTServer,
    writeCharacteristic,
    notifyCharacteristic,
    writes: [] as Uint8Array[],
    disconnect: vi.fn(),
    emitNotify(payload: unknown) {
      const bytes = new TextEncoder().encode(JSON.stringify(payload));
      Object.defineProperty(notifyCharacteristic, "value", {
        configurable: true,
        value: new DataView(bytes.buffer),
      });
      notifyCharacteristic.dispatchEvent(new Event("characteristicvaluechanged"));
    },
  } satisfies BleUartConnection & {
    writes: Uint8Array[];
    emitNotify(payload: unknown): void;
  };
  return connection;
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}
