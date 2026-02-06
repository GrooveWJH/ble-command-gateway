"""Shared BLE runtime helpers for Bless-based servers."""

from __future__ import annotations

import asyncio
import importlib
import inspect
from typing import Any, Awaitable, cast


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
