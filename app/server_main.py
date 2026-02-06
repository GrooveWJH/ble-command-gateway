"""Server application entrypoint."""

from __future__ import annotations

import argparse
import asyncio
import logging
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[1]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from ble.server_gateway import BLEProvisioningServer
from config.defaults import DEFAULT_CONNECT_TIMEOUT, DEFAULT_DEVICE_NAME
from server.preflight import format_preflight_report, run_preflight_checks


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="BLE-based Wi-Fi provisioning server")
    parser.add_argument("--device-name", default=DEFAULT_DEVICE_NAME, help="BLE advertised name")
    parser.add_argument("--ifname", default=None, help="Wi-Fi interface name, e.g. wlan0")
    parser.add_argument("--adapter", default="hci0", help="Bluetooth adapter, e.g. hci0 (empty for auto)")
    parser.add_argument("--connect-timeout", type=int, default=DEFAULT_CONNECT_TIMEOUT, help="nmcli connect timeout seconds")
    parser.add_argument("--log-level", default="INFO", choices=["DEBUG", "INFO", "WARNING", "ERROR"])
    return parser.parse_args()


async def main() -> None:
    args = parse_args()
    logging.basicConfig(
        level=getattr(logging, args.log_level),
        format="%(asctime)s %(levelname)s %(name)s | %(message)s",
    )

    adapter = args.adapter.strip() or "hci0"
    preflight = run_preflight_checks(adapter)
    print(format_preflight_report(preflight))
    if not preflight.ok:
        raise SystemExit("Preflight failed. Resolve the checks above before starting BLE advertising.")

    server = BLEProvisioningServer(
        device_name=args.device_name,
        interface=args.ifname,
        connect_timeout=args.connect_timeout,
        adapter=adapter,
    )
    try:
        await server.start()
    finally:
        await server.stop()


def run() -> int:
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n[server] interrupted")
    return 0


if __name__ == "__main__":
    raise SystemExit(run())
