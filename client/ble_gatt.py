"""Shared GATT helpers for BLE clients."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any, cast

from bleak import BleakClient


class ServiceShapeIssue(Enum):
    MISSING_SERVICE = "missing_service"
    MISSING_CHARACTERISTIC = "missing_characteristic"


@dataclass(frozen=True)
class ServiceShapeError:
    issue: ServiceShapeIssue
    uuid: str


class WriteMode(Enum):
    WITH_RESPONSE = "with_response"
    WITH_RESPONSE_ASSUME_SENT = "with_response(assume_sent)"
    WITHOUT_RESPONSE = "without_response"
    WITHOUT_RESPONSE_STICKY = "without_response(sticky)"


@dataclass(frozen=True)
class WriteConfig:
    allow_no_response: bool
    assume_sent_on_ack_quirk: bool = True


@dataclass(frozen=True)
class WriteState:
    sticky_no_response: bool = False


@dataclass(frozen=True)
class WriteOutcome:
    mode: WriteMode
    next_state: WriteState
    detail: str | None = None


def _normalize_property(value: Any) -> str:
    return str(value).strip().lower().replace("_", "-").replace(" ", "-")


def supports_write_without_response(props: list[Any]) -> bool:
    normalized = {_normalize_property(p) for p in props}
    return any("write-without-response" in item for item in normalized)


def is_ack_quirk_error(message: str) -> bool:
    return "Code=14" in message or "Unlikely error" in message


def is_cbatt_error(message: str) -> bool:
    return "CBATTErrorDomain" in message


async def resolve_services(client: BleakClient) -> Any | None:
    services = client.services
    if services is not None:
        return services

    getter = getattr(client, "get_services", None)
    if callable(getter):
        return await cast(Any, getter)()
    return None


def verify_service_shape(
    services: Any,
    service_uuid: str,
    required_chars: list[str],
) -> ServiceShapeError | None:
    service = services.get_service(service_uuid)
    if service is None:
        return ServiceShapeError(ServiceShapeIssue.MISSING_SERVICE, service_uuid)

    for char_uuid in required_chars:
        if service.get_characteristic(char_uuid) is None:
            return ServiceShapeError(ServiceShapeIssue.MISSING_CHARACTERISTIC, char_uuid)
    return None


def describe_service_shape_error(error: ServiceShapeError, *, zh_cn: bool) -> str:
    if zh_cn:
        if error.issue is ServiceShapeIssue.MISSING_SERVICE:
            return f"未发现目标服务 UUID: {error.uuid}"
        return f"目标设备缺少特征: {error.uuid}"

    if error.issue is ServiceShapeIssue.MISSING_SERVICE:
        return f"missing service: {error.uuid}"
    return f"missing characteristic: {error.uuid}"


async def write_with_strategy(
    client: BleakClient,
    char_uuid: str,
    payload: bytes,
    *,
    config: WriteConfig,
    state: WriteState,
) -> WriteOutcome:
    if state.sticky_no_response and config.allow_no_response:
        await client.write_gatt_char(char_uuid, payload, response=False)
        return WriteOutcome(WriteMode.WITHOUT_RESPONSE_STICKY, state)

    try:
        await client.write_gatt_char(char_uuid, payload, response=True)
        return WriteOutcome(WriteMode.WITH_RESPONSE, WriteState(False))
    except Exception as exc:  # noqa: BLE001
        message = str(exc)
        if is_ack_quirk_error(message) and config.assume_sent_on_ack_quirk:
            return WriteOutcome(WriteMode.WITH_RESPONSE_ASSUME_SENT, state, detail=message)
        if not is_cbatt_error(message) or not config.allow_no_response:
            raise

    await client.write_gatt_char(char_uuid, payload, response=False)
    return WriteOutcome(WriteMode.WITHOUT_RESPONSE, WriteState(True))
