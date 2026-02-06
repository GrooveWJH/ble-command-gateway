"""Whitelisted system command execution helpers."""

from __future__ import annotations

import asyncio
import json
import os
import re
import shutil
import subprocess
from dataclasses import dataclass

from protocol.command_ids import CMD_NET_IFCONFIG, CMD_SYS_WHOAMI, CMD_WIFI_SCAN

STATUS_PROBE_WIFI = "__status.wifi"
STATUS_PROBE_HOSTNAME = "__status.hostname"
STATUS_PROBE_USER = "__status.user"
STATUS_PROBE_SSH = "__status.ssh"
STATUS_PROBE_SYSTEM = "__status.system"


@dataclass(frozen=True)
class SystemExecResult:
    ok: bool
    text: str


def _resolve_ifconfig_bin() -> str | None:
    for candidate in ("ifconfig", "/sbin/ifconfig", "/usr/sbin/ifconfig"):
        if candidate.startswith("/"):
            path = candidate
        else:
            path = shutil.which(candidate)
        if path and os.path.isfile(path) and os.access(path, os.X_OK):
            return path
    return None


def _resolve_ip_bin() -> str | None:
    for candidate in ("ip", "/sbin/ip", "/usr/sbin/ip", "/bin/ip", "/usr/bin/ip"):
        if candidate.startswith("/"):
            path = candidate
        else:
            path = shutil.which(candidate)
        if path and os.path.isfile(path) and os.access(path, os.X_OK):
            return path
    return None


async def run_named_command(name: str, ifname: str | None, timeout_sec: float) -> SystemExecResult:
    if name == CMD_SYS_WHOAMI:
        return await _run_effective_user(timeout_sec)
    if name == CMD_NET_IFCONFIG:
        ifconfig_bin = _resolve_ifconfig_bin()
        if ifconfig_bin is None:
            return SystemExecResult(False, "ifconfig command not found")
        cmd = [ifconfig_bin]
        if ifname:
            cmd.append(ifname)
        return await _run(cmd, timeout_sec)
    if name == CMD_WIFI_SCAN:
        return await _run_wifi_scan(ifname)
    if name == STATUS_PROBE_USER:
        return await _run_effective_user(timeout_sec)
    if name == STATUS_PROBE_HOSTNAME:
        return await _run(["hostname"], timeout_sec)
    if name == STATUS_PROBE_SYSTEM:
        return await _run(["uname", "-srm"], timeout_sec)
    if name == STATUS_PROBE_WIFI:
        return await _probe_wifi_status(ifname, timeout_sec)
    if name == STATUS_PROBE_SSH:
        return await _probe_ssh_status(timeout_sec)
    return SystemExecResult(False, f"unsupported system command: {name}")


async def _run(cmd: list[str], timeout_sec: float) -> SystemExecResult:
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout_sec)
    except asyncio.TimeoutError:
        proc.kill()
        await proc.communicate()
        return SystemExecResult(False, f"system command timeout after {timeout_sec:.1f}s")

    out = stdout.decode("utf-8", errors="ignore").strip()
    err = stderr.decode("utf-8", errors="ignore").strip()
    text = out or err or f"rc={proc.returncode}"
    if len(text) > 2000:
        text = text[:2000] + "...(truncated)"
    return SystemExecResult(proc.returncode == 0, text)


async def _run_wifi_scan(ifname: str | None) -> SystemExecResult:
    # Trigger active scan and wait briefly for fresh scan results.
    rescan_cmd = ["nmcli", "device", "wifi", "rescan"]
    if ifname:
        rescan_cmd += ["ifname", ifname]
    rescan = await _run(rescan_cmd, timeout_sec=6.0)
    if not rescan.ok:
        return rescan

    await asyncio.sleep(5)

    list_cmd = ["nmcli", "-t", "-f", "IN-USE,BSSID,SSID,CHAN,RATE,SIGNAL,BARS,SECURITY", "device", "wifi", "list"]
    if ifname:
        list_cmd += ["ifname", ifname]
    listed = await _run(list_cmd, timeout_sec=8.0)
    if not listed.ok:
        return listed

    entries = _parse_wifi_scan_entries(listed.text)

    payload = {
        "ifname": ifname or "",
        "count": len(entries),
        "aps": entries,
    }
    text = json.dumps(payload, ensure_ascii=False)
    if len(text) > 2000:
        text = text[:2000] + "...(truncated)"
    return SystemExecResult(True, text)


async def _run_effective_user(timeout_sec: float) -> SystemExecResult:
    # When server is started via sudo, prefer original operator account.
    sudo_user = os.environ.get("SUDO_USER", "").strip()
    if sudo_user:
        return SystemExecResult(True, sudo_user)
    return await _run(["whoami"], timeout_sec)


async def _probe_wifi_status(ifname: str | None, timeout_sec: float) -> SystemExecResult:
    status = await _run(["nmcli", "-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device", "status"], timeout_sec)
    if not status.ok:
        return status

    wifi_seen = False
    for raw in status.text.splitlines():
        line = raw.strip()
        if not line:
            continue
        parts = line.split(":", 3)
        if len(parts) < 4:
            continue
        device, dev_type, dev_state, connection = parts
        if dev_type != "wifi":
            continue
        if ifname and device != ifname:
            continue
        wifi_seen = True
        if dev_state == "connected" and connection and connection != "--":
            ip = _ipv4_for_interface(device)
            return SystemExecResult(
                True,
                json.dumps(
                    {
                        "state": "connected",
                        "ssid": connection,
                        "device": device,
                        "ip": ip,
                    },
                    ensure_ascii=False,
                ),
            )
    if wifi_seen:
        return SystemExecResult(True, json.dumps({"state": "disconnected"}, ensure_ascii=False))
    return SystemExecResult(True, json.dumps({"state": "no_wifi_device"}, ensure_ascii=False))


async def _probe_ssh_status(timeout_sec: float) -> SystemExecResult:
    for service in ("ssh", "sshd"):
        enabled_out = await _run(["systemctl", "is-enabled", service], timeout_sec)
        active_out = await _run(["systemctl", "is-active", service], timeout_sec)
        merged_text = f"{enabled_out.text} | {active_out.text}".lower()
        if "not-found" in merged_text or "could not be found" in merged_text:
            continue
        return SystemExecResult(
            True,
            f"service={service},enabled={enabled_out.text},active={active_out.text}",
        )
    return SystemExecResult(True, "service_not_found")


def _ipv4_for_interface(interface: str) -> str | None:
    ip_bin = _resolve_ip_bin()
    if ip_bin is not None:
        try:
            out = subprocess.check_output([ip_bin, "-4", "-o", "addr", "show", interface], text=True).strip()
        except Exception:
            out = ""
        for line in out.splitlines():
            parts = line.split()
            if "inet" not in parts:
                continue
            idx = parts.index("inet")
            if idx + 1 >= len(parts):
                continue
            token = parts[idx + 1]
            return token.split("/", 1)[0]

    try:
        out = subprocess.check_output(["nmcli", "-g", "IP4.ADDRESS", "device", "show", interface], text=True).strip()
    except Exception:
        return None
    return _extract_ipv4(out)


def _extract_ipv4(text: str) -> str | None:
    for line in text.splitlines():
        candidate = line.split(":", 1)[-1].strip()
        match = re.search(r"\b(?:\d{1,3}\.){3}\d{1,3}\b", candidate)
        if match:
            return match.group(0)
    return None


def _parse_wifi_scan_entries(text: str) -> list[dict[str, str | int]]:
    # nmcli -t -f IN-USE,BSSID,SSID,CHAN,RATE,SIGNAL,BARS,SECURITY output.
    # Keep non-empty SSID rows only, sort descending by signal.
    rows: list[dict[str, str | int]] = []
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        parts = _split_nmcli_terse_fields(line, expected_fields=8)
        if len(parts) < 8:
            continue
        _ = _unescape_nmcli_terse(parts[0]).strip() in {"*", "yes", "true"}
        _ = _unescape_nmcli_terse(parts[1]).strip() or "--"
        ssid = _unescape_nmcli_terse(parts[2]).strip()
        if not ssid:
            continue
        chan = _unescape_nmcli_terse(parts[3]).strip() or "-"
        _ = _unescape_nmcli_terse(parts[4]).strip() or "-"
        try:
            signal = int(parts[5].strip())
        except ValueError:
            continue
        signal = max(0, min(100, signal))
        _ = _unescape_nmcli_terse(parts[6]).strip() or "-"
        _ = _unescape_nmcli_terse(parts[7]).strip() or "--"
        rows.append(
            {
                "ssid": ssid,
                "chan": chan,
                "signal": signal,
            }
        )

    rows.sort(key=lambda item: (-int(item["signal"]), str(item["ssid"]).lower(), str(item["chan"]).lower()))
    return rows


def _split_nmcli_terse_fields(line: str, expected_fields: int) -> list[str]:
    fields: list[str] = []
    buf: list[str] = []
    escaped = False
    for ch in line:
        if escaped:
            buf.append(ch)
            escaped = False
            continue
        if ch == "\\":
            escaped = True
            continue
        if ch == ":" and len(fields) < expected_fields - 1:
            fields.append("".join(buf))
            buf = []
            continue
        buf.append(ch)
    fields.append("".join(buf))
    return fields


def _unescape_nmcli_terse(value: str) -> str:
    return value.replace("\\:", ":").replace("\\\\", "\\")
