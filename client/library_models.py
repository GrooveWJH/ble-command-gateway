"""Public models for reusable BLE gateway client APIs."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Protocol, TypeAlias

from client.models import ResultCode, RunResult


class GatewayErrorCode(str, Enum):
    INVALID_ARGUMENT = "invalid_argument"
    NOT_FOUND = "not_found"
    CONNECT_FAILED = "connect_failed"
    DISCONNECTED = "disconnected"
    TIMEOUT = "timeout"
    COMMAND_FAILED = "command_failed"
    BLE_ERROR = "ble_error"


@dataclass
class GatewayError(Exception):
    code: GatewayErrorCode
    message: str
    retryable: bool = False
    raw: str | None = None

    def __str__(self) -> str:
        return self.message


@dataclass(frozen=True)
class DeviceInfo:
    name: str | None
    address: str
    adv_name: str | None = None
    adv_uuids: tuple[str, ...] = ()
    raw: Any | None = field(default=None, repr=False, compare=False)

    @property
    def label(self) -> str:
        display_name = self.adv_name or self.name or "<NoName>"
        if self.adv_uuids:
            return f"{display_name} | {self.address} | {','.join(self.adv_uuids)}"
        return f"{display_name} | {self.address}"

    @classmethod
    def from_any(cls, device: Any) -> "DeviceInfo":
        if isinstance(device, cls):
            return device
        adv_uuids = tuple(getattr(device, "adv_uuids", None) or ())
        return cls(
            name=getattr(device, "name", None),
            address=str(getattr(device, "address", "")),
            adv_name=getattr(device, "adv_name", None),
            adv_uuids=adv_uuids,
            raw=device,
        )


class DeviceLike(Protocol):
    address: str
    name: str | None


DeviceRef: TypeAlias = DeviceInfo | DeviceLike | str


@dataclass(frozen=True)
class ScanSnapshot:
    devices: tuple[DeviceInfo, ...]
    matched: tuple[DeviceInfo, ...]
    total_count: int


@dataclass(frozen=True)
class CommandResult:
    code: ResultCode
    message: str
    command: str
    data: dict[str, Any] | None = None

    @property
    def ok(self) -> bool:
        return self.code is ResultCode.SUCCESS

    def to_run_result(self) -> RunResult:
        return RunResult(self.code, self.message, data=self.data)


@dataclass(frozen=True)
class ProvisionResult:
    code: ResultCode
    message: str
    ip: str | None = None
    ssh_user: str | None = None

    @property
    def ok(self) -> bool:
        return self.code is ResultCode.SUCCESS

    def to_run_result(self) -> RunResult:
        return RunResult(
            self.code,
            self.message,
            ip=self.ip,
            ssh_user=self.ssh_user,
        )


@dataclass(frozen=True)
class StatusResult:
    code: ResultCode
    message: str

    @property
    def ok(self) -> bool:
        return self.code is ResultCode.SUCCESS

    def to_run_result(self) -> RunResult:
        return RunResult(self.code, self.message)
