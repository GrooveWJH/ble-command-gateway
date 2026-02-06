#!/usr/bin/env python3
"""Live BLE scan utility for incremental device observation.

This is a runtime test helper (not a pytest test):
- continuously scans BLE advertisements
- prints NEW / UPDATED devices incrementally
- prints fields used by provisioning match logic
"""

import argparse
import asyncio
import json
import signal
import sys
import time
from pathlib import Path
from typing import Any

from bleak import BleakScanner

ROOT_DIR = Path(__file__).resolve().parents[2]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from config.ble_uuid import SERVICE_UUID  # noqa: E402
from config.defaults import DEFAULT_DEVICE_NAME  # noqa: E402


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Continuously scan BLE devices and print incremental updates")
    parser.add_argument("--target-name", default=DEFAULT_DEVICE_NAME, help="Target name substring to match")
    parser.add_argument("--target-service-uuid", default=SERVICE_UUID, help="Target Service UUID to match")
    parser.add_argument("--refresh", type=float, default=0.25, help="Refresh interval seconds")
    return parser.parse_args()


def _normalize_service_uuids(value: Any) -> list[str]:
    if not value:
        return []
    return [str(x).lower() for x in value]


def _snapshot(device: Any, adv: Any, target_name: str, target_service_uuid: str) -> dict[str, Any]:
    name = str(device.name or "")
    address = str(device.address)
    service_uuids = _normalize_service_uuids(getattr(adv, "service_uuids", None))
    local_name = str(getattr(adv, "local_name", "") or "")
    rssi = int(getattr(device, "rssi", -9999))

    return {
        "address": address,
        "name": name,
        "local_name": local_name,
        "rssi": rssi,
        "service_uuids": service_uuids,
        "manufacturer_data_keys": sorted([hex(int(k)) for k in getattr(adv, "manufacturer_data", {}).keys()]),
        "match_name": bool(name and target_name in name),
        "match_service_uuid": target_service_uuid.lower() in service_uuids,
    }


def _print_snapshot(prefix: str, snap: dict[str, Any]) -> None:
    ts = time.strftime("%H:%M:%S")
    print(
        f"[{ts}] {prefix:<7} addr={snap['address']} rssi={snap['rssi']} "
        f"name={snap['name']!r} local_name={snap['local_name']!r} "
        f"match_name={snap['match_name']} match_service_uuid={snap['match_service_uuid']} "
        f"service_uuids={snap['service_uuids']} manufacturer_keys={snap['manufacturer_data_keys']}"
    )


async def main() -> None:
    args = parse_args()

    stop = asyncio.Event()

    def _handle_stop(_sig: int, _frame: Any) -> None:
        stop.set()

    signal.signal(signal.SIGINT, _handle_stop)
    signal.signal(signal.SIGTERM, _handle_stop)

    latest: dict[str, dict[str, Any]] = {}
    last_fingerprint: dict[str, str] = {}

    def _fingerprint(entry: dict[str, Any]) -> str:
        return json.dumps(entry, sort_keys=True, ensure_ascii=False)

    def _on_detect(device: Any, adv: Any) -> None:
        snap = _snapshot(device, adv, args.target_name, args.target_service_uuid)
        address = snap["address"]
        fp = _fingerprint(snap)

        # Hard de-dup: never print the exact same entry twice.
        if last_fingerprint.get(address) == fp:
            return

        prev = latest.get(address)
        latest[address] = snap
        last_fingerprint[address] = fp

        if prev is None:
            _print_snapshot("NEW", snap)
            return

        _print_snapshot("UPDATED", snap)

    scanner = BleakScanner(detection_callback=_on_detect)
    await scanner.start()
    print(
        "Live BLE scan started. "
        "Press Ctrl+C to stop. "
        f"target_name={args.target_name!r}, target_service_uuid={args.target_service_uuid}"
    )

    try:
        while not stop.is_set():
            await asyncio.sleep(args.refresh)
    finally:
        await scanner.stop()
        print(f"Scan stopped. observed_devices={len(latest)}")


if __name__ == "__main__":
    asyncio.run(main())
