import { writeWithResponse, type BleUartConnection } from "../ble/webBluetooth";

export class BleWriteQueue {
  private queue: Promise<unknown> = Promise.resolve();

  constructor(private readonly connection: BleUartConnection) {}

  enqueue(bytes: Uint8Array): Promise<void> {
    const write = this.queue
      .catch(() => undefined)
      .then(() => writeWithResponse(this.connection.writeCharacteristic, bytes));
    this.queue = write;
    return write;
  }

  writeCharacteristic(): BluetoothRemoteGATTCharacteristic {
    const writeValue = (value: BufferSource) => this.enqueue(bufferSourceBytes(value));
    return {
      ...this.connection.writeCharacteristic,
      writeValueWithResponse: writeValue,
      writeValue,
    } as BluetoothRemoteGATTCharacteristic;
  }
}

function bufferSourceBytes(value: BufferSource): Uint8Array {
  return ArrayBuffer.isView(value)
    ? new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
    : new Uint8Array(value);
}
