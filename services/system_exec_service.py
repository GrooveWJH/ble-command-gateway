"""Whitelisted system command execution helpers."""

from __future__ import annotations

import asyncio
import os
import shutil
from dataclasses import dataclass

from protocol.command_ids import CMD_NET_IFCONFIG, CMD_SYS_WHOAMI


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


async def run_named_command(name: str, ifname: str | None, timeout_sec: float) -> SystemExecResult:
    if name == CMD_SYS_WHOAMI:
        return await _run(["whoami"], timeout_sec)
    if name == CMD_NET_IFCONFIG:
        ifconfig_bin = _resolve_ifconfig_bin()
        if ifconfig_bin is None:
            return SystemExecResult(False, "ifconfig command not found")
        cmd = [ifconfig_bin]
        if ifname:
            cmd.append(ifname)
        return await _run(cmd, timeout_sec)
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
