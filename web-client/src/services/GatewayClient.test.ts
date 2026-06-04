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

  it("waits for the final event ack write before starting the next command", async () => {
    const connection = createConnection();
    const ackGate = createDeferred<void>();
    connection.blockNextAck(ackGate.promise);
    const client = new GatewayClient(connection, { debugMode: "off" });
    const statusPromise = client.runCommand("system.status");
    let statusResolved = false;
    statusPromise.then(() => {
      statusResolved = true;
    });
    const capabilitiesPromise = client.runCommand("system.capabilities");

    await flushMicrotasks();
    expect(connection.writes).toHaveLength(1);
    const statusRequest = decodeWrite(connection.writes[0]);
    connection.emitNotify({
      id: statusRequest.id,
      cmd: "system.status",
      phase: "result",
      seq: 1,
      final: true,
      ok: true,
      code: "OK",
      text: "status ready",
      v: "YundroneBT-V2.1.0",
    });
    await flushMicrotasks();

    expect(connection.writes.map((write) => decodeWrite(write).cmd)).toEqual([
      "system.status",
      "link.ack",
    ]);
    await flushMicrotasks();
    expect(statusResolved).toBe(false);

    ackGate.resolve();
    await expect(statusPromise).resolves.toMatchObject({ cmd: "system.status" });
    await flushMicrotasks();

    expect(connection.writes.map((write) => decodeWrite(write).cmd)).toEqual([
      "system.status",
      "link.ack",
      "system.capabilities",
    ]);
    const capabilitiesRequest = decodeWrite(connection.writes[2]);
    connection.emitNotify({
      id: capabilitiesRequest.id,
      cmd: "system.capabilities",
      phase: "result",
      seq: 1,
      final: true,
      ok: true,
      code: "OK",
      text: "capabilities ready",
      v: "YundroneBT-V2.1.0",
    });

    await expect(capabilitiesPromise).resolves.toMatchObject({ cmd: "system.capabilities" });
  });
});

function createConnection() {
  const notifyCharacteristic = new EventTarget() as BluetoothRemoteGATTCharacteristic;
  let blockedAck: Promise<void> | undefined;
  const writeCharacteristic = {
    writeValueWithResponse: vi.fn(async (value: BufferSource) => {
      const bytes = new Uint8Array(value as ArrayBuffer);
      connection.writes.push(bytes);
      if (decodeWrite(bytes).cmd === "link.ack" && blockedAck) {
        const wait = blockedAck;
        blockedAck = undefined;
        await wait;
      }
    }),
  } as unknown as BluetoothRemoteGATTCharacteristic;
  const device = new EventTarget() as BluetoothDevice;
  Object.assign(device, { name: "yundrone-lab1-k9x8" });

  const connection = {
    device,
    server: {} as BluetoothRemoteGATTServer,
    writeCharacteristic,
    notifyCharacteristic,
    writes: [] as Uint8Array[],
    blockNextAck(promise: Promise<void>) {
      blockedAck = promise;
    },
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
    blockNextAck(promise: Promise<void>): void;
    emitNotify(payload: unknown): void;
  };
  return connection;
}

function decodeWrite(write: Uint8Array) {
  return JSON.parse(new TextDecoder().decode(write)) as { id: string; cmd: string };
}

function createDeferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });
  return { promise, resolve };
}

async function flushMicrotasks(): Promise<void> {
  for (let index = 0; index < 8; index += 1) {
    await Promise.resolve();
  }
}
