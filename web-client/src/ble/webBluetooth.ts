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
  const userAgent = navigator.userAgent;
  const isEdge = /\bEdg\//.test(userAgent);
  const isLinux = /Linux/.test(userAgent);
  const isWindows = /Windows NT/.test(userAgent);
  const base = {
    secureContext,
    hasBluetoothApi,
    userAgent,
  };
  if (!secureContext) {
    return {
      ...base,
      supported: false,
      reasonKey: "support.reasonSecureContext",
    };
  }
  if (!hasBluetoothApi) {
    return {
      ...base,
      supported: false,
      reasonKey: isEdge && isLinux
        ? "support.reasonLinuxEdge"
        : isWindows
          ? "support.reasonWindowsBluetooth"
          : "support.reasonNoBluetooth",
    };
  }
  return { ...base, supported: true };
}

export function isStableYundroneName(name: string, prefix = "yundrone"): boolean {
  const escaped = escapeRegExp(prefix);
  const direct = new RegExp(`^${escaped}-(.+)$`);
  const bracketed = new RegExp(`\\[(${escaped}-.+?)\\]`);
  const directMatch = name.match(direct);
  if (directMatch) return isValidYundroneSuffix(directMatch[1]);
  const bracketedMatch = name.match(bracketed);
  if (!bracketedMatch) return false;
  const suffix = bracketedMatch[1].slice(prefix.length + 1);
  return isValidYundroneSuffix(suffix);
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
    throw new Error(`Selected device is not a YunDrone device name: ${displayName}`);
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

function isValidYundroneSuffix(suffix: string): boolean {
  if (!suffix || suffix.split("-").length > 2) return false;
  return suffix
    .split("-")
    .every((part) => part.length > 0 && /^[0-9a-z]+$/.test(part));
}
