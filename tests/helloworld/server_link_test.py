#!/usr/bin/env python3
import argparse
import asyncio
import importlib
import logging
import sys
from pathlib import Path
from typing import Any

ROOT_DIR = Path(__file__).resolve().parents[2]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from config import CHAR_READ_UUID, CHAR_WRITE_UUID, SERVICE_UUID  # noqa: E402


def _load_bless_symbols() -> tuple[type[Any], Any, Any]:
    module = importlib.import_module("bless")
    bless_server_cls = getattr(module, "BlessServer")
    gatt_props = getattr(module, "GATTCharacteristicProperties")
    gatt_perms = getattr(module, "GATTAttributePermissions")
    return bless_server_cls, gatt_props, gatt_perms


class HelloLinkServer:
    def __init__(self, device_name: str) -> None:
        self.device_name = device_name
        self.server: Any | None = None
        self.logger = logging.getLogger("server_link_test")

    async def start(self) -> None:
        bless_server_cls, gatt_props, gatt_perms = _load_bless_symbols()

        server = bless_server_cls(name=self.device_name, loop=asyncio.get_running_loop())
        await server.add_new_service(SERVICE_UUID)

        await server.add_new_characteristic(
            SERVICE_UUID,
            CHAR_WRITE_UUID,
            properties=gatt_props.write,
            permissions=gatt_perms.writeable,
            value=bytearray(),
        )
        await server.add_new_characteristic(
            SERVICE_UUID,
            CHAR_READ_UUID,
            properties=gatt_props.read | gatt_props.notify,
            permissions=gatt_perms.readable,
            value=bytearray("hello from server | standby", "utf-8"),
        )

        write_char = server.get_characteristic(CHAR_WRITE_UUID)
        if write_char is None:
            raise RuntimeError("write characteristic setup failed")
        setattr(write_char, "write_request_func", self._on_write)

        await server.start()
        self.server = server
        self.logger.info("BLE server started: %s", self.device_name)
        print(f"[server] advertising as: {self.device_name}")
        print("[server] waiting for client message...")

        while True:
            await asyncio.sleep(1)

    def _on_write(self, _characteristic: Any, value: Any, **_kwargs: Any) -> None:
        recv_text = bytes(value).decode("utf-8", errors="ignore").strip() or "<empty>"
        print(f"[server] received from client: {recv_text}")
        reply = f"hello from server_link_test.py | got: {recv_text}"
        self._publish_reply(reply)

    def _publish_reply(self, text: str) -> None:
        if self.server is None:
            return
        read_char = self.server.get_characteristic(CHAR_READ_UUID)
        if read_char is None:
            self.logger.error("read characteristic missing")
            return

        read_char.value = bytearray(text, "utf-8")
        print(f"[server] sent to client: {text}")
        try:
            self.server.update_value(SERVICE_UUID, CHAR_READ_UUID)
        except Exception as exc:  # noqa: BLE001
            self.logger.debug("notify failed: %s", exc)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="BLE hello-world link server")
    parser.add_argument("--device-name", default="BLE_Hello_Server", help="BLE advertised name")
    parser.add_argument(
        "--log-level",
        default="INFO",
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        help="logging level",
    )
    return parser.parse_args()


async def main() -> None:
    args = parse_args()
    logging.basicConfig(
        level=getattr(logging, args.log_level),
        format="%(asctime)s %(levelname)s %(name)s | %(message)s",
    )

    app = HelloLinkServer(device_name=args.device_name)
    await app.start()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n[server] stopped")
