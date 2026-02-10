from __future__ import annotations

import json
import re
import string
from typing import Callable, Iterable, Protocol

from client.gateway_result_adapter import GatewayResultAdapter
from client.library_api import SyncBleGatewayClient, SyncSessionHandle
from client.library_models import DeviceInfo, GatewayError, ScanSnapshot
from client.models import ResultCode
from client.gui.state import TaskResult
from protocol.command_ids import CMD_HELP, CMD_PING, CMD_STATUS, CMD_WIFI_SCAN

ALLOWED_DIAGNOSTIC_COMMANDS: frozenset[str] = frozenset({CMD_STATUS, CMD_WIFI_SCAN, CMD_PING, CMD_HELP})
STATUS_DISPLAY_ORDER: tuple[str, ...] = ("Wi-Fi", "IP", "User", "Hostname", "SSH", "System")


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


def _safe_int_from_any(value: object, *, default: int = 0) -> int:
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str):
        text = value.strip()
        if not text:
            return default
        try:
            return int(text)
        except ValueError:
            return default
    return default


def _gateway_error_to_task_result(exc: GatewayError) -> TaskResult:
    run_result = GatewayResultAdapter.from_gateway_error(exc)
    return TaskResult(
        ok=False,
        code=run_result.code,
        message=run_result.message,
    )


def _unknown_error_to_task_result(exc: Exception) -> TaskResult:
    message = str(exc).strip() or repr(exc)
    return TaskResult(
        ok=False,
        code=ResultCode.FAILED,
        message=f"{type(exc).__name__}: {message}",
    )


def _select_display_devices(snapshot: ScanSnapshot) -> list[DeviceInfo]:
    if snapshot.matched:
        return list(snapshot.matched)
    named = [device for device in snapshot.devices if (device.adv_name or device.name)]
    return named


def scan_devices(
    gateway: SyncBleGatewayClient,
    *,
    target_name: str,
    timeout: int,
    on_progress: Callable[[float, float, int, int], None] | None = None,
    on_detect: Callable[[DeviceInfo], None] | None = None,
    stop_event: StopSignal | None = None,
) -> TaskResult:
    try:
        snapshot = gateway.scan_snapshot(
            target_name=target_name,
            timeout=timeout,
            on_progress=on_progress,
            on_detect=on_detect,
            stop_event=stop_event,
        )
    except GatewayError as exc:
        return _gateway_error_to_task_result(exc)
    except Exception as exc:  # noqa: BLE001
        return _unknown_error_to_task_result(exc)

    display_devices = _select_display_devices(snapshot)
    auto_selected: DeviceInfo | None = None
    if len(snapshot.matched) == 1:
        auto_selected = snapshot.matched[0]
    elif not snapshot.matched and len(display_devices) == 1:
        auto_selected = display_devices[0]

    payload = {
        "all_devices": list(snapshot.devices),
        "matched_devices": list(snapshot.matched),
        "display_devices": display_devices,
        "total_count": snapshot.total_count,
        "auto_selected": auto_selected,
        "auto_connect_device": snapshot.matched[0] if snapshot.matched else None,
    }

    if snapshot.matched:
        msg = f"扫描完成：总设备 {snapshot.total_count}，匹配 {len(snapshot.matched)}"
    elif display_devices:
        msg = f"扫描完成：无过滤匹配，展示 {len(display_devices)} 个命名设备"
    else:
        msg = f"扫描完成：总设备 {snapshot.total_count}，未发现可选设备"

    return TaskResult(
        ok=True,
        code=ResultCode.SUCCESS,
        message=msg,
        payload=payload,
    )


def connect_selected(
    gateway: SyncBleGatewayClient,
    device: DeviceInfo,
) -> TaskResult:
    try:
        session = gateway.connect(device)
    except GatewayError as exc:
        return _gateway_error_to_task_result(exc)
    except Exception as exc:  # noqa: BLE001
        return _unknown_error_to_task_result(exc)

    return TaskResult(
        ok=True,
        code=ResultCode.SUCCESS,
        message="已连接并验证服务",
        payload={"session": session, "device": device},
    )


def provision_current(
    session: SyncSessionHandle,
    *,
    ssid: str,
    password: str,
    timeout: int,
    verbose: bool,
) -> TaskResult:
    if not ssid.strip():
        return TaskResult(ok=False, code=ResultCode.INPUT_ERROR, message="SSID 不能为空")
    if not _is_valid_wifi_password(password):
        return TaskResult(
            ok=False,
            code=ResultCode.INPUT_ERROR,
            message="Wi-Fi 密码不合法：留空(开放网络)，或 8-63 位字符，或 64 位十六进制。",
        )

    try:
        result = session.provision(ssid=ssid, password=password, timeout=timeout, verbose=verbose)
    except GatewayError as exc:
        return _gateway_error_to_task_result(exc)
    except Exception as exc:  # noqa: BLE001
        return _unknown_error_to_task_result(exc)

    payload: dict[str, object] = {}
    if result.ip:
        payload["ip"] = result.ip
    if result.ssh_user:
        payload["ssh_user"] = result.ssh_user
    return TaskResult(
        ok=result.ok,
        code=result.code,
        message=result.message,
        payload=payload,
    )


def _parse_wifi_scan_payload(message: str) -> list[dict[str, object]] | None:
    try:
        payload = json.loads(message)
    except Exception:
        return None
    if not isinstance(payload, dict):
        return None
    rows = payload.get("aps")
    if not isinstance(rows, list):
        return None
    normalized: list[dict[str, object]] = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        ssid = str(row.get("ssid", "")).strip()
        if not ssid:
            continue
        chan = str(row.get("chan", "-")).strip() or "-"
        try:
            signal = _safe_int_from_any(row.get("signal", 0), default=0)
        except Exception:
            signal = 0
        normalized.append({"ssid": ssid, "chan": chan, "signal": max(0, min(signal, 100))})
    return normalized


def _normalize_status_key(raw_key: str) -> str:
    compact = raw_key.strip().lower().replace(" ", "")
    mapping = {
        "wi-fi": "Wi-Fi",
        "wifi": "Wi-Fi",
        "ip": "IP",
        "user": "User",
        "hostname": "Hostname",
        "ssh": "SSH",
        "system": "System",
    }
    return mapping.get(compact, raw_key.strip() or "Unknown")


def _extract_wifi_embedded_ip(value: str) -> tuple[str, str | None]:
    match = re.search(r",\s*IP=([^,)]+)", value)
    if match is None:
        return value, None
    ip = match.group(1).strip() or "unknown"
    wifi_value = f"{value[:match.start()]}{value[match.end():]}".strip()
    wifi_value = wifi_value.replace("(,", "(").replace(", )", ")")
    if wifi_value.endswith(","):
        wifi_value = wifi_value[:-1].rstrip()
    return wifi_value or "unknown", ip


def _parse_status_payload(message: str) -> list[dict[str, str]] | None:
    parsed_map: dict[str, str] = {}
    for line in message.splitlines():
        raw = line.strip()
        if not raw or ":" not in raw:
            continue
        key, value = raw.split(":", 1)
        normalized_key = _normalize_status_key(key)
        normalized_value = value.strip() or "-"
        if normalized_key == "Wi-Fi":
            wifi_value, embedded_ip = _extract_wifi_embedded_ip(normalized_value)
            parsed_map["Wi-Fi"] = wifi_value
            if embedded_ip and "IP" not in parsed_map:
                parsed_map["IP"] = embedded_ip
            continue
        parsed_map[normalized_key] = normalized_value

    if not parsed_map:
        return None

    rows: list[dict[str, str]] = []
    for key in STATUS_DISPLAY_ORDER:
        if key in parsed_map:
            rows.append({"key": key, "value": parsed_map.pop(key)})
    for key in sorted(parsed_map.keys()):
        rows.append({"key": key, "value": parsed_map[key]})
    return rows


def _parse_status_data_payload(data: object) -> list[dict[str, str]] | None:
    if not isinstance(data, dict):
        return None
    status_obj = data.get("status")
    if not isinstance(status_obj, dict):
        return None

    normalized_map: dict[str, str] = {}
    for raw_key, raw_value in status_obj.items():
        normalized_key = _normalize_status_key(str(raw_key))
        normalized_map[normalized_key] = str(raw_value).strip() or "-"

    if not normalized_map:
        return None

    rows: list[dict[str, str]] = []
    for key in STATUS_DISPLAY_ORDER:
        if key in normalized_map:
            rows.append({"key": key, "value": normalized_map.pop(key)})
    for key in sorted(normalized_map.keys()):
        rows.append({"key": key, "value": normalized_map[key]})
    return rows


def _aggregate_wifi_scan_rows(rows: list[dict[str, object]]) -> list[dict[str, object]]:
    strongest_by_ssid: dict[str, dict[str, object]] = {}
    for row in rows:
        ssid = str(row.get("ssid", "")).strip()
        if not ssid:
            continue
        chan = str(row.get("chan", "-")).strip() or "-"
        try:
            signal = _safe_int_from_any(row.get("signal", 0), default=0)
        except Exception:
            signal = 0
        signal = max(0, min(signal, 100))
        candidate: dict[str, object] = {"ssid": ssid, "chan": chan, "signal": signal}
        current = strongest_by_ssid.get(ssid)
        candidate_signal = _safe_int_from_any(candidate.get("signal", 0), default=0)
        current_signal = _safe_int_from_any(None if current is None else current.get("signal", 0), default=0)
        if current is None or candidate_signal > current_signal:
            strongest_by_ssid[ssid] = candidate

    return sorted(
        strongest_by_ssid.values(),
        key=lambda item: (-_safe_int_from_any(item.get("signal", 0), default=0), str(item.get("ssid", "")).lower()),
    )


def run_diagnostic(
    session: SyncSessionHandle,
    *,
    command_id: str,
    timeout: int,
) -> TaskResult:
    if command_id not in ALLOWED_DIAGNOSTIC_COMMANDS:
        return TaskResult(
            ok=False,
            code=ResultCode.INPUT_ERROR,
            message=f"不支持的诊断命令: {command_id}",
        )

    try:
        result = session.run_command(command=command_id, args={}, timeout=timeout)
    except GatewayError as exc:
        return _gateway_error_to_task_result(exc)
    except Exception as exc:  # noqa: BLE001
        return _unknown_error_to_task_result(exc)

    payload: dict[str, object] = {"command": command_id, "raw_text": result.message}
    if command_id == CMD_STATUS and result.ok:
        status_rows = _parse_status_data_payload(result.data)
        if status_rows is None:
            status_rows = _parse_status_payload(result.message)
        payload["status_rows"] = status_rows
    if command_id == CMD_WIFI_SCAN and result.ok:
        rows = _parse_wifi_scan_payload(result.message)
        if rows is not None:
            payload["wifi_scan_rows"] = rows
            payload["wifi_scan_aggregated_rows"] = _aggregate_wifi_scan_rows(rows)
    return TaskResult(
        ok=result.ok,
        code=result.code,
        message=result.message,
        payload=payload,
    )


def run_heartbeat(
    session: SyncSessionHandle,
    *,
    timeout: int = 5,
) -> TaskResult:
    try:
        result = session.run_command(command=CMD_PING, args={}, timeout=timeout, reporter=None)
    except GatewayError as exc:
        return _gateway_error_to_task_result(exc)
    except Exception as exc:  # noqa: BLE001
        return _unknown_error_to_task_result(exc)

    if result.ok:
        return TaskResult(
            ok=True,
            code=result.code,
            message=result.message or "pong",
            payload={"command": CMD_PING},
        )
    return TaskResult(
        ok=False,
        code=result.code,
        message=result.message or "心跳失败",
        payload={"command": CMD_PING},
    )


def close_session(session: SyncSessionHandle | None) -> TaskResult:
    if session is None:
        return TaskResult(ok=True, code=ResultCode.SUCCESS, message="会话已关闭")
    try:
        session.close()
    except GatewayError as exc:
        return _gateway_error_to_task_result(exc)
    except Exception as exc:  # noqa: BLE001
        return _unknown_error_to_task_result(exc)
    return TaskResult(ok=True, code=ResultCode.SUCCESS, message="会话已关闭")


def make_device_table_rows(devices: Iterable[DeviceInfo]) -> list[list[str]]:
    rows: list[list[str]] = []
    for device in devices:
        name = device.adv_name or device.name or "<NoName>"
        uuids = ",".join(device.adv_uuids) if device.adv_uuids else "-"
        rows.append([name, device.address, uuids])
    return rows
