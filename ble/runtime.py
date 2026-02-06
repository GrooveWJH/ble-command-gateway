"""Shared BLE runtime helpers for Bless-based servers."""

from __future__ import annotations

import asyncio
import importlib
import inspect
from typing import Any, Awaitable, Callable, cast


def load_bless_symbols() -> tuple[type[Any], Any, Any]:
    module = importlib.import_module("bless")
    bless_server_cls = getattr(module, "BlessServer")
    gatt_props = getattr(module, "GATTCharacteristicProperties")
    gatt_perms = getattr(module, "GATTAttributePermissions")
    return bless_server_cls, gatt_props, gatt_perms


def write_properties(gatt_props: Any) -> Any:
    props = gatt_props.write
    cmd_prop = getattr(gatt_props, "write_without_response", None)
    if cmd_prop is not None:
        props |= cmd_prop
    return props


async def stop_bless_server(server: Any, timeout: float = 5.0) -> None:
    stop_fn = getattr(server, "stop", None)
    if not callable(stop_fn):
        return

    result = stop_fn()
    if inspect.isawaitable(result):
        await asyncio.wait_for(cast(Awaitable[Any], result), timeout=timeout)


def try_patch_bluez_advertising_interval(
    app: Any,
    *,
    min_interval_ms: int,
    max_interval_ms: int,
) -> Callable[[], None] | None:
    """Patch BlueZ advertisement start to apply Min/MaxInterval (experimental)."""
    try:
        from bless.backends.bluezdbus.dbus.advertisement import BlueZLEAdvertisement, Type  # type: ignore
    except Exception:
        return None

    original = getattr(app, "start_advertising", None)
    if not callable(original):
        return None

    async def _start_advertising(adapter: Any) -> None:
        await app.set_name(adapter, app.app_name)
        advertisement = BlueZLEAdvertisement(Type.PERIPHERAL, len(app.advertisements) + 1, app)
        advertisement.MinInterval = int(min_interval_ms)
        advertisement.MaxInterval = int(max_interval_ms)
        app.advertisements.append(advertisement)
        advertisement._service_uuids.append(app.services[0].UUID)
        app.bus.export(advertisement.path, advertisement)
        iface = adapter.get_interface("org.bluez.LEAdvertisingManager1")
        await iface.call_register_advertisement(advertisement.path, {})  # type: ignore

    app.start_advertising = _start_advertising

    def _restore() -> None:
        app.start_advertising = original

    return _restore
