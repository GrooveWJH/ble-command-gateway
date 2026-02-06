#!/usr/bin/env python3
"""Hello-world BLE link test server."""

from __future__ import annotations

import argparse
import asyncio
import logging
from collections import OrderedDict
from typing import Any, Callable

from config.ble_uuid import CHAR_READ_UUID, CHAR_WRITE_UUID, SERVICE_UUID
from protocol.link import build_server_reply, extract_seq_token
from common.reporting import make_reporter, show_panel, show_table
from ble.runtime import load_bless_symbols, stop_bless_server, write_properties
from server.preflight import detect_default_adapter, run_preflight_checks

Reporter = Callable[[str], None]


def _hint_advertise_failure(exc: Exception, adapter: str | None) -> RuntimeError:
    adapter_name = adapter or "<auto>"
    message = str(exc)
    detail = [
        f"Failed to register BLE advertisement on adapter {adapter_name}: {message}",
        "Actionable checks:",
        "1) Run with privileges: sudo -E $(pwd)/.venv/bin/python tests/integration/server_link_test.py --adapter hci0",
        "2) Ensure adapter is up: sudo hciconfig hci0 up",
        "3) Check LE advertising support: bluetoothctl show",
        "4) Restart bluetooth service if stale adv exists: sudo systemctl restart bluetooth",
        "5) Verify no other process is occupying advertisement slots",
    ]
    return RuntimeError("\n".join(detail))


class HelloLinkServer:
    def __init__(self, device_name: str, adapter: str | None, reporter: Reporter = print) -> None:
        self.device_name = device_name
        self.adapter = adapter
        self.server: Any | None = None
        self.logger = logging.getLogger("server_link_test")
        self._reply_cache: OrderedDict[str, str] = OrderedDict()
        self.reporter = reporter
        self._client_active = False
        self._paneler = None
        self._table_builder = None

    def set_paneler(self, paneler) -> None:
        self._paneler = paneler

    def set_table_builder(self, table_builder) -> None:
        self._table_builder = table_builder

    def _report(self, message: str) -> None:
        self.reporter(message)

    async def start(self) -> None:
        self._report("[server] loading bless symbols...")
        bless_server_cls, gatt_props, gatt_perms = load_bless_symbols()

        self._report(f"[server] creating BlessServer(name={self.device_name!r}, adapter={self.adapter or 'auto'})")
        server = bless_server_cls(
            name=self.device_name,
            loop=asyncio.get_running_loop(),
            adapter=self.adapter,
        )

        self._report(f"[server] adding service: {SERVICE_UUID}")
        await server.add_new_service(SERVICE_UUID)

        self._report(f"[server] adding write characteristic: {CHAR_WRITE_UUID}")
        await server.add_new_characteristic(
            SERVICE_UUID,
            CHAR_WRITE_UUID,
            properties=write_properties(gatt_props),
            permissions=gatt_perms.writeable,
            value=bytearray(),
        )

        self._report(f"[server] adding read/notify characteristic: {CHAR_READ_UUID}")
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

        setattr(server, "write_request_func", self._on_write)
        self._report("[server] write callback bound")

        self._report("[server] starting advertisement...")
        try:
            await server.start()
        except Exception as exc:  # noqa: BLE001
            raise _hint_advertise_failure(exc, self.adapter) from exc

        self.server = server
        self.logger.info("BLE server started: %s", self.device_name)
        self._report(f"[server] advertising as: {self.device_name}")
        self._report(f"[server] adapter: {self.adapter or 'auto'}")
        self._report("[server] waiting for client message...")

        while True:
            await asyncio.sleep(1)

    async def stop(self) -> None:
        if self.server is None:
            return

        server = self.server
        self.server = None

        self._report("[server] stopping advertisement...")
        if self._client_active:
            self._report("[red][server] client disconnected (server stopping)[/red]")
            self._client_active = False
        try:
            await stop_bless_server(server, timeout=5)
            self._report("[server] advertisement stopped")
        except Exception as exc:  # noqa: BLE001
            self.logger.warning("BLE shutdown failed: %s", exc)
            self._report(f"[server] stop failed: {exc}")

    def _on_write(self, _characteristic: Any, value: Any, **_kwargs: Any) -> None:
        raw = bytes(value)
        recv_text = raw.decode("utf-8", errors="ignore").strip() or "<empty>"
        if not self._client_active:
            self._client_active = True
            self._report("[green][server] client connected (first write)[/green]")
        token = extract_seq_token(recv_text)
        token_display = token or "-"

        cached_reply = self._reply_cache.get(token) if token else None
        if cached_reply is not None:
            reply = cached_reply
            self._report(f"[server] duplicate token detected, replay cached reply token={token}")
        else:
            reply = build_server_reply(recv_text)
            if token:
                self._reply_cache[token] = reply
                while len(self._reply_cache) > 64:
                    self._reply_cache.popitem(last=False)

        self._publish_reply(reply, token_display, len(raw), recv_text)

    def _publish_reply(self, text: str, token: str, recv_size: int, recv_text: str) -> None:
        if self.server is None:
            self._report("[server] skip publish: server is None")
            return

        read_char = self.server.get_characteristic(CHAR_READ_UUID)
        if read_char is None:
            self.logger.error("read characteristic missing")
            return

        payload = bytearray(text, "utf-8")
        read_char.value = payload

        try:
            show_table(
                self.reporter,
                self._paneler,
                self._table_builder,
                title="Server Exchange",
                columns=["Dir", "Seq", "Bytes", "Text"],
                rows=[
                    ["<- recv", token, str(recv_size), recv_text],
                    ["-> send", token, str(len(payload)), text],
                ],
                style="cyan",
                wrap_panel=False,
            )
            self.server.update_value(SERVICE_UUID, CHAR_READ_UUID)
        except Exception as exc:  # noqa: BLE001
            self.logger.debug("notify failed: %s", exc)
            self._report(f"[server] notify failed: {exc}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="BLE hello-world link server")
    parser.add_argument("--device-name", default="BLE_Hello_Server", help="BLE advertised name")
    parser.add_argument("--adapter", default="auto", help="Bluetooth adapter (auto, hci0, hci1)")
    parser.add_argument(
        "--log-level",
        default="INFO",
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        help="logging level",
    )
    return parser.parse_args()


async def run() -> None:
    args = parse_args()
    logging.basicConfig(
        level=getattr(logging, args.log_level),
        format="%(asctime)s %(levelname)s %(name)s | %(message)s",
    )

    reporter, paneler, table_builder = make_reporter()
    show_panel(paneler, "BLE link test server", "BLE Link Test", "cyan")

    adapter = args.adapter.strip()
    if adapter in {"", "auto"}:
        detected = detect_default_adapter()
        if detected is None:
            raise SystemExit("No bluetooth adapter detected (no hci* found)")
        adapter = detected
    preflight = run_preflight_checks(adapter)
    rows = [[("PASS" if item.ok else "FAIL"), item.name, item.detail] for item in preflight.checks]
    show_table(
        reporter,
        paneler,
        table_builder,
        title="Preflight",
        columns=["Status", "Check", "Detail"],
        rows=rows,
        style="cyan",
    )
    if not preflight.ok:
        raise SystemExit("Preflight failed. Resolve the checks above before starting BLE advertising.")

    app = HelloLinkServer(device_name=args.device_name, adapter=adapter, reporter=reporter)
    app.set_paneler(paneler)
    app.set_table_builder(table_builder)
    try:
        await app.start()
    finally:
        await app.stop()


def main() -> int:
    try:
        asyncio.run(run())
    except KeyboardInterrupt:
        print("\n[server] interrupted")
    return 0
