#!/usr/bin/env python3
import argparse
import asyncio
import sys
from pathlib import Path
from typing import Any, cast

from bleak import BleakClient, BleakScanner

ROOT_DIR = Path(__file__).resolve().parents[2]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from config import CHAR_READ_UUID, CHAR_WRITE_UUID, SERVICE_UUID  # noqa: E402


def _notify_handler(_: Any, data: bytearray) -> None:
    text = bytes(data).decode("utf-8", errors="ignore").strip() or "<empty>"
    print(f"\n[client] notify from server: {text}")


async def find_target_device(target_name: str, timeout: int) -> Any | None:
    print(f"[client] scanning for '{target_name}' ({timeout}s)...")
    devices = await BleakScanner.discover(timeout=timeout)

    matches = [d for d in devices if d.name and target_name in d.name]
    matches.sort(key=lambda d: (d.name or "", d.address))

    if not matches:
        print("[client] no matching device found")
        return None

    if len(matches) > 1:
        print(f"[client] found {len(matches)} matches, using first one")
    device = matches[0]
    print(f"[client] selected: {device.name} ({device.address})")
    return device


async def run_once(target_name: str, timeout: int) -> int:
    device = await find_target_device(target_name, timeout)
    if device is None:
        return 2

    hello = "hello from client_link_test.py"
    print("[client] connecting...")
    try:
        async with BleakClient(device) as client:
            services = client.services
            if services is None:
                get_services = getattr(client, "get_services", None)
                if callable(get_services):
                    services = await cast(Any, get_services)()
                else:
                    print("[client] unable to read GATT services")
                    return 3

            if services.get_service(SERVICE_UUID) is None:
                print(f"[client] missing service: {SERVICE_UUID}")
                return 3

            await client.start_notify(CHAR_READ_UUID, _notify_handler)
            await client.write_gatt_char(CHAR_WRITE_UUID, hello.encode("utf-8"), response=True)
            print(f"[client] sent to server: {hello}")

            for _ in range(10):
                data = await client.read_gatt_char(CHAR_READ_UUID)
                text = bytes(data).decode("utf-8", errors="ignore").strip() or "<empty>"
                print(f"[client] read from server: {text}")
                if "hello from server_link_test.py" in text:
                    print("[client] link test success")
                    await client.stop_notify(CHAR_READ_UUID)
                    return 0
                await asyncio.sleep(1)

            await client.stop_notify(CHAR_READ_UUID)
            print("[client] timeout waiting server hello")
            return 4
    except Exception as exc:  # noqa: BLE001
        print(f"[client] BLE error: {exc}")
        return 5


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="BLE hello-world link client")
    parser.add_argument("--target-name", default="BLE_Hello_Server", help="target BLE device name filter")
    parser.add_argument("--scan-timeout", type=int, default=15, help="scan timeout in seconds")
    return parser.parse_args()


async def main() -> int:
    args = parse_args()
    return await run_once(args.target_name, args.scan_timeout)


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
