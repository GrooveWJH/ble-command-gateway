"""Shared BLE scan helpers for CLI clients."""

from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Any, Callable

from bleak import BleakScanner

DeviceCallback = Callable[[Any], None]
MatchCallback = Callable[[Any], None]
ProgressCallback = Callable[[float, float, int, int], None]
ScannerCallback = Callable[[BleakScanner | None], None]


@dataclass(frozen=True)
class ScanResult:
    devices: list[Any]
    matched: list[Any]
    stopped_early: bool


def device_name_matches(device: Any, target_name: str, *, match_all: bool) -> bool:
    if match_all:
        return True
    return bool(device.name and target_name in device.name)


def filter_devices_by_name(devices: list[Any], target_name: str, *, match_all: bool) -> list[Any]:
    filtered = [d for d in devices if device_name_matches(d, target_name, match_all=match_all)]
    filtered.sort(key=lambda d: (d.name or "", d.address))
    return filtered


async def scan_devices(
    target_name: str,
    timeout: float,
    *,
    match_all: bool,
    refresh_interval: float = 0.1,
    stop_on_first_match: bool = False,
    on_detect: DeviceCallback | None = None,
    on_match: MatchCallback | None = None,
    on_progress: ProgressCallback | None = None,
    on_scanner: ScannerCallback | None = None,
) -> ScanResult:
    seen: dict[str, Any] = {}
    matched: dict[str, Any] = {}
    matched_event = asyncio.Event()

    def _on_detect(device: Any, _adv: Any) -> None:
        addr = getattr(device, "address", "")
        if not addr:
            return
        if addr not in seen:
            seen[addr] = device
            if on_detect:
                on_detect(device)

        if device_name_matches(device, target_name, match_all=match_all) and addr not in matched:
            matched[addr] = seen[addr]
            if on_match:
                on_match(seen[addr])
            matched_event.set()

    scanner = BleakScanner(detection_callback=_on_detect)
    if on_scanner:
        on_scanner(scanner)

    await scanner.start()
    loop = asyncio.get_running_loop()
    start = loop.time()
    stopped_early = False

    try:
        while True:
            elapsed = loop.time() - start
            devices_now = len(seen)
            matched_now = len(matched)
            if on_progress:
                on_progress(elapsed, timeout, devices_now, matched_now)

            if elapsed >= timeout:
                break

            if stop_on_first_match and matched_event.is_set():
                stopped_early = True
                break

            wait_time = min(refresh_interval, max(0.0, timeout - elapsed))
            if stop_on_first_match:
                try:
                    await asyncio.wait_for(matched_event.wait(), timeout=wait_time)
                except TimeoutError:
                    pass
            else:
                await asyncio.sleep(wait_time)
    finally:
        await scanner.stop()
        if on_scanner:
            on_scanner(None)

    devices = list(seen.values())
    matches = list(matched.values())
    if not matches:
        matches = filter_devices_by_name(devices, target_name, match_all=match_all)
    else:
        matches = filter_devices_by_name(matches, target_name, match_all=match_all)

    return ScanResult(devices=devices, matched=matches, stopped_early=stopped_early)
