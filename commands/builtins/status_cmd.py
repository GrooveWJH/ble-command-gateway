from __future__ import annotations

import json

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandSpec
from protocol.command_ids import CMD_STATUS
from protocol.envelope import CommandRequest, CommandResponse, response_ok
from services.system_exec_service import (
    STATUS_PROBE_HOSTNAME,
    STATUS_PROBE_SSH,
    STATUS_PROBE_SYSTEM,
    STATUS_PROBE_USER,
    STATUS_PROBE_WIFI,
)

SPEC = CommandSpec(
    name=CMD_STATUS,
    summary="Read server connectivity and system status",
    usage="status",
    permission="user",
    risk="low",
    timeout_sec=15.0,
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(context: DispatchContext, request: CommandRequest) -> CommandResponse:
        async def _probe(name: str) -> tuple[bool, str]:
            return await context.run_system_command(name, None, 3.0)

        wifi_ok, wifi_text = await _probe(STATUS_PROBE_WIFI)
        user_ok, user_text = await _probe(STATUS_PROBE_USER)
        host_ok, host_text = await _probe(STATUS_PROBE_HOSTNAME)
        ssh_ok, ssh_text = await _probe(STATUS_PROBE_SSH)
        system_ok, system_text = await _probe(STATUS_PROBE_SYSTEM)

        wifi_value, ip_value = _format_wifi_and_ip(wifi_ok, wifi_text)
        user_value = _format_field(user_ok, user_text)
        host_value = _format_field(host_ok, host_text)
        ssh_value = _format_ssh(ssh_ok, ssh_text)
        system_value = _format_field(system_ok, system_text)

        lines: list[str] = []
        lines.append(f"Wi-Fi: {wifi_value}")
        lines.append(f"IP: {ip_value}")
        lines.append(f"User: {user_value}")
        lines.append(f"Hostname: {host_value}")
        lines.append(f"SSH: {ssh_value}")
        lines.append(f"System: {system_value}")

        data = {
            "status": {
                "wifi": wifi_value,
                "ip": ip_value,
                "user": user_value,
                "hostname": host_value,
                "ssh": ssh_value,
                "system": system_value,
            }
        }
        return response_ok(request.request_id, "\n".join(lines), data=data)

    dispatcher.register(SPEC, _handler)


def _format_field(ok: bool, text: str) -> str:
    if not ok:
        return f"unknown ({text})"
    stripped = text.strip()
    return stripped if stripped else "unknown"


def _format_wifi(ok: bool, text: str) -> str:
    wifi_value, _ = _format_wifi_and_ip(ok, text)
    return wifi_value


def _format_ip(ok: bool, text: str) -> str:
    _, ip_value = _format_wifi_and_ip(ok, text)
    return ip_value


def _format_wifi_and_ip(ok: bool, text: str) -> tuple[str, str]:
    if not ok:
        return f"unknown ({text})", "unknown"
    try:
        payload = json.loads(text)
    except Exception:
        payload = None
    if isinstance(payload, dict):
        state = str(payload.get("state", "")).strip()
        if state == "connected":
            ssid = str(payload.get("ssid", "")).strip() or "unknown"
            ip = str(payload.get("ip", "")).strip() or "unknown"
            return f"connected (SSID={ssid})", ip
        if state == "disconnected":
            return "disconnected", "unknown"
        if state == "no_wifi_device":
            return "no wifi device", "unknown"
    if text.startswith("connected:"):
        ssid = text.split(":", 1)[1]
        return f"connected (SSID={ssid})", "unknown"
    if text == "disconnected":
        return "disconnected", "unknown"
    if text == "no_wifi_device":
        return "no wifi device", "unknown"
    return text, "unknown"


def _format_ssh(ok: bool, text: str) -> str:
    if not ok:
        return f"unknown ({text})"
    if text == "service_not_found":
        return "service not found"
    # text format: service=<name>,enabled=<x>,active=<y>
    details: dict[str, str] = {}
    for token in text.split(","):
        if "=" not in token:
            continue
        key, value = token.split("=", 1)
        details[key.strip()] = value.strip()
    service = details.get("service", "ssh")
    enabled = details.get("enabled", "unknown")
    active = details.get("active", "unknown")
    return f"{service} (enabled={enabled}, active={active})"
