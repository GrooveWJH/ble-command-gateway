"""Client package for interactive BLE Wi-Fi provisioning."""

from __future__ import annotations

from typing import Any

__all__ = [
    "BleGatewayClient",
    "SessionHandle",
    "SyncBleGatewayClient",
    "SyncSessionHandle",
    "DeviceInfo",
    "DeviceRef",
    "ScanSnapshot",
    "CommandResult",
    "ProvisionResult",
    "StatusResult",
    "GatewayError",
    "GatewayErrorCode",
]

_LAZY_EXPORTS: dict[str, tuple[str, str]] = {
    "BleGatewayClient": ("client.library_api", "BleGatewayClient"),
    "SessionHandle": ("client.library_api", "SessionHandle"),
    "SyncBleGatewayClient": ("client.library_api", "SyncBleGatewayClient"),
    "SyncSessionHandle": ("client.library_api", "SyncSessionHandle"),
    "CommandResult": ("client.library_models", "CommandResult"),
    "DeviceInfo": ("client.library_models", "DeviceInfo"),
    "DeviceRef": ("client.library_models", "DeviceRef"),
    "GatewayError": ("client.library_models", "GatewayError"),
    "GatewayErrorCode": ("client.library_models", "GatewayErrorCode"),
    "ProvisionResult": ("client.library_models", "ProvisionResult"),
    "ScanSnapshot": ("client.library_models", "ScanSnapshot"),
    "StatusResult": ("client.library_models", "StatusResult"),
}


def __getattr__(name: str) -> Any:
    export = _LAZY_EXPORTS.get(name)
    if export is not None:
        module_name, attr_name = export
        module = __import__(module_name, fromlist=[attr_name])
        return getattr(module, attr_name)
    raise AttributeError(name)
