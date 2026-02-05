"""Shared BLE transport helpers for CLI clients."""

from __future__ import annotations

import asyncio
import time
from dataclasses import dataclass, field
from typing import Any, Awaitable, Callable, TypeVar, cast

from bleak import BleakClient, BleakScanner

from client.ble_scan import scan_devices
from common.reporting import PanelPrinter, TableBuilder

T = TypeVar("T")
Reporter = Callable[[str], None]


class StepTimeoutError(RuntimeError):
    pass


@dataclass
class RuntimeState:
    scanner: BleakScanner | None = None
    clients: set[BleakClient] = field(default_factory=set)
    reporter: Reporter = print
    paneler: PanelPrinter | None = None
    table_builder: TableBuilder | None = None

    async def cleanup(self) -> None:
        scanner = self.scanner
        if scanner is not None:
            self.scanner = None
            try:
                self.reporter("[client] cleanup | stopping scanner")
                await asyncio.wait_for(scanner.stop(), timeout=2.0)
            except Exception as exc:  # noqa: BLE001
                self.reporter(f"[client] cleanup | scanner stop failed: {type(exc).__name__}: {exc}")

        for client in list(self.clients):
            if not client.is_connected:
                self.clients.discard(client)
                continue
            try:
                self.reporter("[client] cleanup | disconnecting client")
                await asyncio.wait_for(client.disconnect(), timeout=5.0)
            except Exception as exc:  # noqa: BLE001
                self.reporter(f"[client] cleanup | disconnect failed: {type(exc).__name__}: {exc}")
            finally:
                self.clients.discard(client)


def summarize_exception(exc: BaseException) -> str:
    subs = getattr(exc, "exceptions", None)
    if isinstance(subs, tuple) and subs and isinstance(subs[0], BaseException):
        return f"{type(exc).__name__}[{len(subs)}]: {summarize_exception(subs[0])}"
    return f"{type(exc).__name__}: {str(exc).strip() or '<no message>'}"


async def run_step(step: str, timeout: float, coro: Awaitable[T], reporter: Reporter = print) -> T:
    start = time.monotonic()
    reporter(f"[client] step start | {step} timeout={timeout:.1f}s")
    try:
        result = await asyncio.wait_for(coro, timeout=timeout)
    except TimeoutError as exc:
        raise StepTimeoutError(
            f"step timeout | {step} elapsed={time.monotonic() - start:.2f}s limit={timeout:.1f}s"
        ) from exc

    reporter(f"[client] step done  | {step} elapsed={time.monotonic() - start:.2f}s")
    return result


def make_disconnected_handler(reporter: Reporter = print) -> Callable[[BleakClient], None]:
    def _handler(_: BleakClient) -> None:
        reporter("\n[client] disconnected callback fired")

    return _handler


async def find_target_device(
    target_name: str,
    timeout: int,
    runtime: RuntimeState,
    *,
    reporter: Reporter = print,
) -> Any | None:
    reporter(f"[client] scan start | target_name={target_name!r} timeout={timeout}s")
    last_remaining = timeout + 1
    match_all = not target_name

    def on_detect(device: Any) -> None:
        reporter(f"[client] discovered | name={device.name!r} addr={device.address}")

    def on_match(device: Any) -> None:
        reporter(f"[client] matched | name={device.name!r} addr={device.address}")

    def on_progress(elapsed: float, total: float, total_devices: int, matched_devices: int) -> None:
        nonlocal last_remaining
        remaining = max(int(total - elapsed), 0)
        if remaining == last_remaining:
            return
        last_remaining = remaining
        reporter(
            f"\r[client] scanning... remaining={remaining:>2}s total={total_devices:>3} matched={matched_devices:>3}",
        )

    result = await scan_devices(
        target_name=target_name,
        timeout=float(timeout),
        match_all=match_all,
        stop_on_first_match=True,
        on_detect=on_detect,
        on_match=on_match,
        on_progress=on_progress,
        on_scanner=lambda scanner: setattr(runtime, "scanner", scanner),
    )
    reporter("")
    if result.stopped_early and result.matched:
        reporter("[client] early stop: matched device found")

    reporter(f"[client] scan done | total={len(result.devices)} matched={len(result.matched)}")
    if not result.matched:
        reporter("[client] no matching device found")
        return None

    selected = result.matched[0]
    reporter(f"[client] selected | name={selected.name!r} addr={selected.address}")
    return selected


async def refresh_device(device: Any, timeout: float, reporter: Reporter = print) -> Any:
    finder = getattr(BleakScanner, "find_device_by_address", None)
    address = getattr(device, "address", "")
    if not callable(finder) or not address:
        return device

    refreshed = await cast(Any, finder)(address, timeout=timeout)
    if refreshed is None:
        reporter(f"[client] refresh skipped: device not rediscovered addr={address}")
        return device

    reporter(f"[client] refreshed target | name={refreshed.name!r} addr={refreshed.address}")
    return refreshed
