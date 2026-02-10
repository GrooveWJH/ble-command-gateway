"""Reusable high-level BLE gateway client APIs."""

from __future__ import annotations

import asyncio
import concurrent.futures
import string
import threading
from dataclasses import dataclass
from typing import Any, Callable, Coroutine, Protocol, TypeVar

from bleak import BleakClient

from client.command_client import (
    close_device_session,
    discover_devices_with_progress,
    open_device_session,
    provision_device,
    run_command,
)
from client.library_models import (
    CommandResult,
    DeviceInfo,
    DeviceRef,
    GatewayError,
    GatewayErrorCode,
    ProvisionResult,
    ScanSnapshot,
    StatusResult,
)
from client.models import ResultCode, RunResult
from config.defaults import (
    DEFAULT_CONNECT_RETRIES,
    DEFAULT_DEVICE_NAME,
    DEFAULT_SCAN_TIMEOUT,
    DEFAULT_WAIT_TIMEOUT,
)
from protocol.command_ids import CMD_STATUS

Reporter = Callable[..., None] | None
_T = TypeVar("_T")
_REPORTER_DEFAULT = object()


class StopSignal(Protocol):
    def is_set(self) -> bool: ...


def _is_valid_wifi_password(password: str) -> bool:
    if password == "":
        return True
    if 8 <= len(password) <= 63:
        return True
    if len(password) == 64 and all(ch in string.hexdigits for ch in password):
        return True
    return False


def _run_result_to_gateway_error(result: RunResult, *, stage: str) -> GatewayError:
    if result.code is ResultCode.TIMEOUT:
        return GatewayError(
            code=GatewayErrorCode.TIMEOUT,
            message=result.message,
            retryable=True,
            raw=stage,
        )
    if result.code is ResultCode.NOT_FOUND:
        return GatewayError(
            code=GatewayErrorCode.NOT_FOUND,
            message=result.message,
            retryable=False,
            raw=stage,
        )
    if stage == "connect":
        return GatewayError(
            code=GatewayErrorCode.CONNECT_FAILED,
            message=result.message,
            retryable=True,
            raw=stage,
        )
    return GatewayError(
        code=GatewayErrorCode.COMMAND_FAILED,
        message=result.message,
        retryable=False,
        raw=stage,
    )


def _coerce_device_info(device: DeviceRef) -> DeviceInfo:
    if isinstance(device, DeviceInfo):
        return device
    if isinstance(device, str):
        address = device.strip()
        if not address:
            raise GatewayError(
                code=GatewayErrorCode.INVALID_ARGUMENT,
                message="device address must not be empty",
                retryable=False,
                raw="device",
            )
        return DeviceInfo(name=None, address=address, raw=address)
    address = str(getattr(device, "address", "")).strip()
    if not address:
        raise GatewayError(
            code=GatewayErrorCode.INVALID_ARGUMENT,
            message="device must have a non-empty address",
            retryable=False,
            raw="device",
        )
    return DeviceInfo.from_any(device)


@dataclass
class SessionHandle:
    """Long-lived connected BLE session for command execution."""

    _device: DeviceInfo
    _client: BleakClient | None
    _wait_timeout: int
    _verbose: bool
    _reporter: Reporter = None

    @property
    def device(self) -> DeviceInfo:
        return self._device

    @property
    def is_connected(self) -> bool:
        if self._client is None:
            return False
        try:
            return bool(self._client.is_connected)
        except Exception:
            return False

    def _connect_target(self) -> DeviceRef:
        return self._device.raw if self._device.raw is not None else self._device.address

    async def run_command(
        self,
        command: str,
        args: dict[str, Any] | None = None,
        timeout: int | None = None,
        reporter: Reporter | object = _REPORTER_DEFAULT,
    ) -> CommandResult:
        if not command.strip():
            raise GatewayError(
                code=GatewayErrorCode.INVALID_ARGUMENT,
                message="command must not be empty",
                retryable=False,
                raw="command",
            )

        resolved_reporter: Reporter
        if reporter is _REPORTER_DEFAULT:
            resolved_reporter = self._reporter
        else:
            resolved_reporter = reporter if callable(reporter) or reporter is None else self._reporter

        result = await run_command(
            self._connect_target(),
            command=command,
            args=args or {},
            wait_timeout=timeout or self._wait_timeout,
            reporter=resolved_reporter,
            client=self._client,
        )
        if self._client is not None and not self.is_connected:
            self._client = None
        return CommandResult(code=result.code, message=result.message, command=command, data=result.data)

    async def provision(
        self,
        ssid: str,
        password: str,
        timeout: int | None = None,
        verbose: bool | None = None,
    ) -> ProvisionResult:
        if not ssid.strip():
            raise GatewayError(
                code=GatewayErrorCode.INVALID_ARGUMENT,
                message="ssid must not be empty",
                retryable=False,
                raw="ssid",
            )
        if not _is_valid_wifi_password(password):
            raise GatewayError(
                code=GatewayErrorCode.INVALID_ARGUMENT,
                message="invalid Wi-Fi password: use empty, 8-63 chars, or 64 hex chars",
                retryable=False,
                raw="password",
            )

        result = await provision_device(
            self._connect_target(),
            ssid=ssid,
            password=password,
            wait_timeout=timeout or self._wait_timeout,
            verbose=self._verbose if verbose is None else verbose,
            reporter=self._reporter,
            client=self._client,
        )
        if self._client is not None and not self.is_connected:
            self._client = None
        return ProvisionResult(
            code=result.code,
            message=result.message,
            ip=result.ip,
            ssh_user=result.ssh_user,
        )

    async def status(self, timeout: int | None = None) -> StatusResult:
        command_result = await self.run_command(CMD_STATUS, args={}, timeout=timeout)
        return StatusResult(
            code=command_result.code,
            message=command_result.message,
        )

    async def close(self) -> None:
        await close_device_session(self._client)
        self._client = None


class BleGatewayClient:
    """High-level async API intended for library consumers and GUI adapters."""

    def __init__(
        self,
        *,
        target_name: str = DEFAULT_DEVICE_NAME,
        scan_timeout: int = DEFAULT_SCAN_TIMEOUT,
        wait_timeout: int = DEFAULT_WAIT_TIMEOUT,
        connect_timeout: int = 10,
        connect_retries: int = DEFAULT_CONNECT_RETRIES,
        verbose: bool = False,
        reporter: Reporter = None,
    ) -> None:
        self.target_name = target_name
        self.scan_timeout = scan_timeout
        self.wait_timeout = wait_timeout
        self.connect_timeout = connect_timeout
        self.connect_retries = connect_retries
        self.verbose = verbose
        self.reporter = reporter

    async def scan(
        self,
        *,
        target_name: str | None = None,
        timeout: int | None = None,
        on_progress: Callable[[float, float, int, int], None] | None = None,
        on_detect: Callable[[DeviceInfo], None] | None = None,
        stop_event: StopSignal | None = None,
    ) -> list[DeviceInfo]:
        snapshot = await self.scan_snapshot(
            target_name=target_name,
            timeout=timeout,
            on_progress=on_progress,
            on_detect=on_detect,
            stop_event=stop_event,
        )
        return list(snapshot.matched)

    async def scan_snapshot(
        self,
        *,
        target_name: str | None = None,
        timeout: int | None = None,
        on_progress: Callable[[float, float, int, int], None] | None = None,
        on_detect: Callable[[DeviceInfo], None] | None = None,
        stop_event: StopSignal | None = None,
    ) -> ScanSnapshot:
        name_filter = self.target_name if target_name is None else target_name
        scan_timeout = self.scan_timeout if timeout is None else timeout
        detect_cb: Callable[[Any], None] | None = None
        if on_detect is not None:
            def _emit_detect(raw_device: Any) -> None:
                on_detect(DeviceInfo.from_any(raw_device))
            detect_cb = _emit_detect
        devices, matched, total = await discover_devices_with_progress(
            target_name=name_filter,
            timeout=scan_timeout,
            on_progress=on_progress,
            on_detect=detect_cb,
            stop_event=stop_event,
        )
        mapped_devices = tuple(DeviceInfo.from_any(d) for d in devices)
        mapped_matched = tuple(DeviceInfo.from_any(d) for d in matched)
        return ScanSnapshot(
            devices=mapped_devices,
            matched=mapped_matched,
            total_count=total,
        )

    async def connect(
        self,
        device: DeviceRef,
        *,
        timeout: int | None = None,
        retries: int | None = None,
    ) -> SessionHandle:
        device_info = _coerce_device_info(device)
        result, client = await open_device_session(
            device_info.raw if device_info.raw is not None else device_info.address,
            timeout=timeout or self.connect_timeout,
            retries=self.connect_retries if retries is None else retries,
            reporter=self.reporter,
        )
        if result.code is not ResultCode.SUCCESS or client is None:
            raise _run_result_to_gateway_error(result, stage="connect")
        return SessionHandle(
            _device=device_info,
            _client=client,
            _wait_timeout=self.wait_timeout,
            _verbose=self.verbose,
            _reporter=self.reporter,
        )


class _BackgroundEventLoop:
    """Owns an asyncio loop in a dedicated thread for sync facade calls."""

    def __init__(self) -> None:
        self._loop = asyncio.new_event_loop()
        self._ready = threading.Event()
        self._closed = False
        self._thread = threading.Thread(target=self._thread_main, daemon=True)
        self._thread.start()
        self._ready.wait()

    def _thread_main(self) -> None:
        asyncio.set_event_loop(self._loop)
        self._ready.set()
        self._loop.run_forever()

    def run(self, coro: Coroutine[Any, Any, _T]) -> _T:
        if self._closed:
            raise RuntimeError("sync client event loop already closed")
        future = asyncio.run_coroutine_threadsafe(coro, self._loop)
        return future.result()

    def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        self._loop.call_soon_threadsafe(self._loop.stop)
        self._thread.join(timeout=5)
        self._loop.close()


class SyncSessionHandle:
    """Synchronous wrapper around :class:`SessionHandle`."""

    def __init__(self, loop_worker: _BackgroundEventLoop, inner: SessionHandle) -> None:
        self._loop_worker = loop_worker
        self._inner = inner

    @property
    def device(self) -> DeviceInfo:
        return self._inner.device

    @property
    def is_connected(self) -> bool:
        return self._inner.is_connected

    def run_command(
        self,
        command: str,
        args: dict[str, Any] | None = None,
        timeout: int | None = None,
        reporter: Reporter | object = _REPORTER_DEFAULT,
    ) -> CommandResult:
        return self._loop_worker.run(
            self._inner.run_command(command=command, args=args, timeout=timeout, reporter=reporter)
        )

    def provision(
        self,
        ssid: str,
        password: str,
        timeout: int | None = None,
        verbose: bool | None = None,
    ) -> ProvisionResult:
        return self._loop_worker.run(
            self._inner.provision(
                ssid=ssid,
                password=password,
                timeout=timeout,
                verbose=verbose,
            )
        )

    def status(self, timeout: int | None = None) -> StatusResult:
        return self._loop_worker.run(self._inner.status(timeout=timeout))

    def close(self) -> None:
        self._loop_worker.run(self._inner.close())


class SyncBleGatewayClient:
    """Thread-safe synchronous facade for GUI frameworks without asyncio ownership."""

    def __init__(self, **kwargs: Any) -> None:
        self._loop_worker = _BackgroundEventLoop()
        self._inner = BleGatewayClient(**kwargs)
        self._sessions: set[SyncSessionHandle] = set()
        self._closed = False

    def _ensure_open(self) -> None:
        if self._closed:
            raise RuntimeError("sync client already closed")

    def _run(self, coro: Coroutine[Any, Any, _T]) -> _T:
        self._ensure_open()
        return self._loop_worker.run(coro)

    def scan(
        self,
        *,
        target_name: str | None = None,
        timeout: int | None = None,
        on_progress: Callable[[float, float, int, int], None] | None = None,
        on_detect: Callable[[DeviceInfo], None] | None = None,
        stop_event: StopSignal | None = None,
    ) -> list[DeviceInfo]:
        self._ensure_open()
        return self._run(
            self._inner.scan(
                target_name=target_name,
                timeout=timeout,
                on_progress=on_progress,
                on_detect=on_detect,
                stop_event=stop_event,
            )
        )

    def scan_snapshot(
        self,
        *,
        target_name: str | None = None,
        timeout: int | None = None,
        on_progress: Callable[[float, float, int, int], None] | None = None,
        on_detect: Callable[[DeviceInfo], None] | None = None,
        stop_event: StopSignal | None = None,
    ) -> ScanSnapshot:
        self._ensure_open()
        return self._run(
            self._inner.scan_snapshot(
                target_name=target_name,
                timeout=timeout,
                on_progress=on_progress,
                on_detect=on_detect,
                stop_event=stop_event,
            )
        )

    def connect(
        self,
        device: DeviceRef,
        *,
        timeout: int | None = None,
        retries: int | None = None,
    ) -> SyncSessionHandle:
        self._ensure_open()
        session = self._run(self._inner.connect(device, timeout=timeout, retries=retries))
        wrapped = SyncSessionHandle(self._loop_worker, session)
        self._sessions.add(wrapped)
        return wrapped

    def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        for session in tuple(self._sessions):
            try:
                session.close()
            except (GatewayError, RuntimeError, concurrent.futures.CancelledError):
                pass
        self._sessions.clear()
        self._loop_worker.close()
