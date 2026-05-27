import type { BrowserSupportState } from "../types";

export const UART_SERVICE_UUID = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
export const UART_WRITE_UUID = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
export const UART_NOTIFY_UUID = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";

export interface BleUartConnection {
  device: BluetoothDevice;
  server: BluetoothRemoteGATTServer;
  writeCharacteristic: BluetoothRemoteGATTCharacteristic;
  notifyCharacteristic: BluetoothRemoteGATTCharacteristic;
  disconnect(): void;
}

export function getBrowserSupport(): BrowserSupportState {
  const secureContext = globalThis.isSecureContext === true;
  const hasBluetoothApi = "bluetooth" in navigator;
  if (!secureContext) {
    return {
      supported: false,
      secureContext,
      hasBluetoothApi,
      reason:
        "当前浏览器无法使用 Web Bluetooth：需要 HTTPS 或 localhost 安全上下文。请使用桌面 Chrome/Edge 或 Android Chrome。",
    };
  }
  if (!hasBluetoothApi) {
    return {
      supported: false,
      secureContext,
      hasBluetoothApi,
      reason: "当前浏览器无法使用 Web Bluetooth，请使用桌面 Chrome/Edge 或 Android Chrome。",
    };
  }
  return { supported: true, secureContext, hasBluetoothApi };
}

export function isStableYundroneName(name: string, prefix = "yundrone"): boolean {
  const escaped = escapeRegExp(prefix);
  const direct = new RegExp(`^${escaped}-[0-9a-z]{6}$`);
  const bracketed = new RegExp(`\\[(${escaped}-[0-9a-z]{6})\\]`);
  return direct.test(name) || bracketed.test(name);
}

export async function requestYundroneDevice(prefix = "yundrone"): Promise<BleUartConnection> {
  const bluetooth = navigator.bluetooth;
  if (!bluetooth) {
    throw new Error("Web Bluetooth is unavailable in this browser");
  }

  const device = await bluetooth.requestDevice({
    filters: [{ namePrefix: `${prefix}-` }, { services: [UART_SERVICE_UUID] }],
    optionalServices: [UART_SERVICE_UUID],
  });
  const displayName = device.name ?? "";
  if (displayName && !isStableYundroneName(displayName, prefix)) {
    throw new Error(`Selected device is not a stable YunDrone identity: ${displayName}`);
  }
  if (!device.gatt) {
    throw new Error("Selected device does not expose GATT");
  }
  const server = await device.gatt.connect();
  const service = await server.getPrimaryService(UART_SERVICE_UUID);
  const writeCharacteristic = await service.getCharacteristic(UART_WRITE_UUID);
  const notifyCharacteristic = await service.getCharacteristic(UART_NOTIFY_UUID);
  await notifyCharacteristic.startNotifications();
  return {
    device,
    server,
    writeCharacteristic,
    notifyCharacteristic,
    disconnect() {
      device.gatt?.disconnect();
    },
  };
}

export async function writeWithResponse(
  characteristic: BluetoothRemoteGATTCharacteristic,
  payload: Uint8Array,
): Promise<void> {
  const buffer = payload.buffer.slice(
    payload.byteOffset,
    payload.byteOffset + payload.byteLength,
  ) as ArrayBuffer;
  if (typeof characteristic.writeValueWithResponse === "function") {
    await characteristic.writeValueWithResponse(buffer);
    return;
  }
  await characteristic.writeValue(buffer);
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
