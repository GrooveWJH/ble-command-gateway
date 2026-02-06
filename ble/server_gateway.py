"""BLE server gateway for command protocol provisioning."""

from __future__ import annotations

import asyncio
import logging
import os
from contextlib import suppress
from typing import Any, Callable

from config.ble_uuid import CHAR_READ_UUID, CHAR_WRITE_UUID, SERVICE_UUID
from config.defaults import FAST_ADV_DURATION_SEC, FAST_ADV_INTERVAL_MS
from protocol.envelope import (
    CODE_BUSY,
    CODE_IN_PROGRESS,
    CODE_INTERNAL_ERROR,
    CODE_PROVISION_FAIL,
    CODE_PROVISION_SUCCESS,
    CommandParseError,
    CommandResponse,
    parse_request,
    response_error,
    response_status,
)
from ble.runtime import load_bless_symbols, stop_bless_server, try_patch_bluez_advertising_interval, write_properties
from ble.response_publisher import ResponsePublisher
from commands.loader import load_builtin_commands
from commands.registry import CommandDispatcher, DispatchContext
from services.system_exec_service import run_named_command
from services.wifi_provisioning_service import WifiProvisioningService


class BLEProvisioningServer:
    def __init__(self, device_name: str, interface: str | None, connect_timeout: int, adapter: str | None) -> None:
        self.device_name = device_name
        self.interface = interface
        self.connect_timeout = connect_timeout
        self.server: Any | None = None
        self.adapter = adapter
        self.logger = logging.getLogger(device_name)
        self._connect_lock = asyncio.Lock()
        self._publisher = ResponsePublisher()
        self._shutdown_requested = asyncio.Event()
        self._adv_reset_task: asyncio.Task[None] | None = None
        self._restore_adv: Callable[[], None] | None = None
        self._provision_task_pending = False
        self._provisioning = WifiProvisioningService(
            interface=interface,
            connect_timeout=connect_timeout,
            logger=self.logger,
        )
        self._dispatcher = CommandDispatcher(
            DispatchContext(
                read_status_text=self._read_status_text,
                start_provision=self._start_provision,
                start_shutdown=self._start_shutdown,
                run_system_command=self._run_system_command,
            ),
            logger=lambda message: self.logger.error("%s", message),
        )
        load_builtin_commands(self._dispatcher)

    async def start(self) -> None:
        try:
            bless_server_cls, gatt_props, gatt_perms = load_bless_symbols()
        except Exception as exc:
            raise RuntimeError("Failed to load bless runtime symbols. Check bless/bleak compatibility.") from exc

        server = bless_server_cls(name=self.device_name, loop=asyncio.get_running_loop(), adapter=self.adapter)

        await server.add_new_service(SERVICE_UUID)
        await server.add_new_characteristic(
            SERVICE_UUID,
            CHAR_WRITE_UUID,
            properties=write_properties(gatt_props),
            permissions=gatt_perms.writeable,
            value=bytearray(),
        )
        await server.add_new_characteristic(
            SERVICE_UUID,
            CHAR_READ_UUID,
            properties=gatt_props.read | gatt_props.notify,
            permissions=gatt_perms.readable,
            value=self._publisher.last_payload,
        )

        self._restore_adv = self._apply_fast_advertising(server)

        write_char = server.get_characteristic(CHAR_WRITE_UUID)
        if write_char is None:
            raise RuntimeError("Write characteristic not found after setup")

        setattr(server, "write_request_func", self._handle_write_request)
        setattr(server, "read_request_func", self._handle_read_request)

        try:
            await server.start()
        except Exception as exc:  # noqa: BLE001
            adapter_name = self.adapter or "<auto>"
            raise RuntimeError(
                "Failed to register BLE advertisement on "
                f"adapter {adapter_name}: {exc}. "
                "Try: sudo, `hciconfig hci0 up`, and `systemctl restart bluetooth`."
            ) from exc

        self.server = server
        self.logger.info("BLE service started as %s", self.device_name)

        if self._restore_adv is not None and FAST_ADV_DURATION_SEC > 0:
            self.logger.info(
                "Using fast advertising interval=%dms for %ds",
                FAST_ADV_INTERVAL_MS,
                FAST_ADV_DURATION_SEC,
            )
            self._adv_reset_task = asyncio.create_task(self._reset_advertising_after_delay())

        await self._shutdown_requested.wait()

    async def stop(self) -> None:
        if self.server is None:
            return

        server = self.server
        self.server = None
        if self._adv_reset_task is not None:
            self._adv_reset_task.cancel()
            with suppress(asyncio.CancelledError):
                await self._adv_reset_task
        self.logger.info("Stopping BLE advertisement")

        stop_fn = getattr(server, "stop", None)
        if not callable(stop_fn):
            self.logger.warning("BlessServer.stop is unavailable; skip shutdown")
            return

        try:
            await stop_bless_server(server, timeout=5)
            self.logger.info("BLE advertisement stopped")
        except Exception as exc:  # noqa: BLE001
            self.logger.warning("BLE shutdown failed: %s", exc)

    def _read_status_text(self) -> str:
        return self._publisher.status_text

    def _handle_read_request(self, _characteristic: Any, **_kwargs: Any) -> bytearray:
        return self._publisher.next_payload(self.logger)

    def _handle_write_request(self, _characteristic: Any, value: Any, **_kwargs: Any) -> None:
        asyncio.create_task(self._process_write(value))

    async def _process_write(self, value: Any) -> None:
        raw_preview = ResponsePublisher.preview_text(self._decode_for_log(value), limit=220)
        self.logger.info("[BLE RX] raw=%s", raw_preview)
        try:
            request = parse_request(value)
            self.logger.info(
                "[BLE RX] id=%s cmd=%s args=%s",
                request.request_id,
                request.command,
                ResponsePublisher.preview_text(str(self._sanitize_args_for_log(request.args))),
            )
        except CommandParseError as exc:
            self.logger.warning("[BLE RX] parse_error code=%s message=%s", exc.code, exc.message)
            self._publish_response(response_error("-", exc.code, exc.message))
            return
        except Exception as exc:  # noqa: BLE001
            self.logger.exception("Unexpected parse failure")
            self._publish_response(response_error("-", CODE_INTERNAL_ERROR, str(exc)))
            return

        try:
            response = await self._dispatcher.dispatch(request)
        except Exception as exc:  # noqa: BLE001
            self.logger.exception("Dispatcher failure")
            response = response_error(request.request_id, CODE_INTERNAL_ERROR, f"{type(exc).__name__}: {exc}")
        self._publish_response(response)

    async def _start_provision(self, request_id: str, ssid: str, password: str) -> bool:
        if self._connect_lock.locked() or self._provision_task_pending:
            self.logger.info(
                "[PROVISION] reject id=%s ssid=%s reason=busy_or_pending lock=%s pending=%s",
                request_id,
                ssid,
                self._connect_lock.locked(),
                self._provision_task_pending,
            )
            return False
        self.logger.info("[PROVISION] accept id=%s ssid=%s", request_id, ssid)
        self._provision_task_pending = True
        asyncio.create_task(self._provision_wifi(request_id, ssid, password))
        return True

    async def _start_shutdown(self, _request_id: str) -> None:
        self._shutdown_requested.set()

    async def _run_system_command(self, command_name: str, ifname: str | None, timeout_sec: float) -> tuple[bool, str]:
        effective_ifname = ifname or self.interface
        result = await run_named_command(command_name, effective_ifname, timeout_sec)
        return result.ok, result.text

    def _publish_response(self, response: CommandResponse) -> None:
        self._publisher.publish(
            response,
            server=self.server,
            service_uuid=SERVICE_UUID,
            read_char_uuid=CHAR_READ_UUID,
            logger=self.logger,
        )

    @staticmethod
    def _sanitize_args_for_log(args: dict[str, Any]) -> dict[str, Any]:
        safe = dict(args)
        if "pwd" in safe:
            safe["pwd"] = "***"
        if "password" in safe:
            safe["password"] = "***"
        return safe

    async def _provision_wifi(self, request_id: str, ssid: str, password: str) -> None:
        started = asyncio.get_running_loop().time()
        heartbeat_stop = asyncio.Event()
        heartbeat_task: asyncio.Task[None] | None = None
        try:
            if self._connect_lock.locked():
                self._publish_response(response_status(request_id, CODE_BUSY, "Provisioning in progress", final=True))
                return

            async with self._connect_lock:
                self.logger.info("[PROVISION] start id=%s ssid=%s", request_id, ssid)
                self._publish_response(response_status(request_id, CODE_IN_PROGRESS, f"Connecting SSID={ssid}", final=False))
                heartbeat_task = asyncio.create_task(self._provision_heartbeat(request_id, ssid, started, heartbeat_stop))
                ok, message, ip = await self._provisioning.connect_and_get_ip(ssid, password, ip_timeout=15)
                heartbeat_stop.set()
                if heartbeat_task is not None:
                    with suppress(asyncio.CancelledError):
                        await heartbeat_task
                if not ok:
                    elapsed = asyncio.get_running_loop().time() - started
                    self.logger.info(
                        "[PROVISION] fail id=%s ssid=%s elapsed=%.1fs reason=%s",
                        request_id,
                        ssid,
                        elapsed,
                        message,
                    )
                    self._publish_response(response_status(request_id, CODE_PROVISION_FAIL, message, final=True))
                    return
                elapsed = asyncio.get_running_loop().time() - started
                self.logger.info(
                    "[PROVISION] success id=%s ssid=%s ip=%s elapsed=%.1fs",
                    request_id,
                    ssid,
                    ip,
                    elapsed,
                )
                self._publish_response(
                    response_status(
                        request_id,
                        CODE_PROVISION_SUCCESS,
                        f"Success_IP:{ip}",
                        final=True,
                        data={"ip": ip, "ssid": ssid, "user": self._effective_server_user()},
                    )
                )
        finally:
            heartbeat_stop.set()
            if heartbeat_task is not None and not heartbeat_task.done():
                heartbeat_task.cancel()
                with suppress(asyncio.CancelledError):
                    await heartbeat_task
            self.logger.debug("[PROVISION] end id=%s pending=False", request_id)
            self._provision_task_pending = False

    async def _provision_heartbeat(self, request_id: str, ssid: str, started: float, stop: asyncio.Event) -> None:
        # Emit in-progress status every second so client has live progress during nmcli/DHCP wait.
        while not stop.is_set():
            await asyncio.sleep(1)
            if stop.is_set():
                break
            elapsed = int(asyncio.get_running_loop().time() - started)
            self._publish_response(
                response_status(
                    request_id,
                    CODE_IN_PROGRESS,
                    f"Connecting SSID={ssid} ({elapsed}s)",
                    final=False,
                )
            )

    @staticmethod
    def _effective_server_user() -> str | None:
        # Prefer sudo operator account when service runs as root.
        for key in ("SUDO_USER", "USER", "LOGNAME"):
            value = os.environ.get(key, "").strip()
            if value:
                return value
        return None

    def _apply_fast_advertising(self, server: Any) -> Callable[[], None] | None:
        if FAST_ADV_DURATION_SEC <= 0 or FAST_ADV_INTERVAL_MS <= 0:
            return None
        app = getattr(server, "app", None)
        if app is None:
            return None
        return try_patch_bluez_advertising_interval(
            app,
            min_interval_ms=FAST_ADV_INTERVAL_MS,
            max_interval_ms=FAST_ADV_INTERVAL_MS,
        )

    async def _reset_advertising_after_delay(self) -> None:
        try:
            await asyncio.sleep(FAST_ADV_DURATION_SEC)
            server = self.server
            if server is None or self._restore_adv is None:
                return
            app = getattr(server, "app", None)
            adapter = getattr(server, "adapter", None)
            if app is None or adapter is None:
                return
            await app.stop_advertising(adapter)
            self._restore_adv()
            await app.start_advertising(adapter)
            self.logger.info("Advertising interval reset to default")
        except asyncio.CancelledError:
            raise
        except Exception as exc:  # noqa: BLE001
            self.logger.warning("Failed to reset advertising interval: %s", exc)

    @staticmethod
    def _decode_for_log(value: Any) -> str:
        try:
            if isinstance(value, memoryview):
                raw = value.tobytes()
            elif isinstance(value, (bytes, bytearray)):
                raw = bytes(value)
            else:
                raw = bytes(value)
            return raw.decode("utf-8", errors="replace")
        except Exception:
            return repr(value)
