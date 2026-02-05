"""BLE advertising preflight checks for Linux + BlueZ."""

from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass
class CheckResult:
    name: str
    ok: bool
    detail: str


@dataclass
class PreflightReport:
    checks: list[CheckResult]

    @property
    def ok(self) -> bool:
        return all(c.ok for c in self.checks)


def _run(cmd: list[str]) -> tuple[int, str, str]:
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    return proc.returncode, proc.stdout.strip(), proc.stderr.strip()


def _parse_hex_or_int(value: str) -> int | None:
    value = value.strip()
    if not value:
        return None
    try:
        if value.lower().startswith("0x"):
            return int(value, 16)
        return int(value)
    except ValueError:
        return None


def _parse_advertising_instances(show_output: str) -> tuple[int | None, int | None]:
    active: int | None = None
    supported: int | None = None
    for raw in show_output.splitlines():
        line = raw.strip()
        if line.startswith("ActiveInstances:"):
            active = _parse_hex_or_int(line.split(":", 1)[1])
        elif line.startswith("SupportedInstances:"):
            supported = _parse_hex_or_int(line.split(":", 1)[1])
    return active, supported


def _check_adapter_exists(adapter: str) -> CheckResult:
    exists = Path("/sys/class/bluetooth") / adapter
    if exists.exists():
        return CheckResult("adapter_exists", True, f"{adapter} exists")
    return CheckResult("adapter_exists", False, f"{adapter} not found under /sys/class/bluetooth")


def _check_adapter_state(adapter: str) -> CheckResult:
    rc, out, err = _run(["hciconfig", adapter])
    if rc != 0:
        detail = err or out or f"hciconfig rc={rc}"
        return CheckResult("adapter_up_running", False, detail)

    first_line = out.splitlines()[0] if out else ""
    up_running = "UP RUNNING" in out
    if up_running:
        return CheckResult("adapter_up_running", True, first_line)
    return CheckResult("adapter_up_running", False, f"{first_line} | missing 'UP RUNNING'")


def _check_bluez_active() -> CheckResult:
    rc, out, err = _run(["systemctl", "is-active", "bluetooth"])
    if rc == 0 and out == "active":
        return CheckResult("bluez_service", True, "bluetooth.service is active")
    detail = err or out or f"systemctl rc={rc}"
    return CheckResult("bluez_service", False, detail)


def _check_advertising_capacity() -> CheckResult:
    rc, out, err = _run(["bluetoothctl", "show"])
    if rc != 0:
        detail = err or out or f"bluetoothctl show rc={rc}"
        return CheckResult("advertising_capacity", False, detail)

    active, supported = _parse_advertising_instances(out)
    if active is None or supported is None:
        return CheckResult(
            "advertising_capacity",
            False,
            "Unable to parse ActiveInstances/SupportedInstances from bluetoothctl show",
        )

    if supported > active:
        return CheckResult(
            "advertising_capacity",
            True,
            f"ActiveInstances={active}, SupportedInstances={supported}",
        )

    return CheckResult(
        "advertising_capacity",
        False,
        f"No free advertising slot: ActiveInstances={active}, SupportedInstances={supported}",
    )


def _check_residual_processes(exclude_pids: set[int] | None = None) -> CheckResult:
    rc, out, err = _run(["ps", "-eo", "pid=,args="])
    if rc != 0:
        detail = err or out or f"ps rc={rc}"
        return CheckResult("residual_server_process", False, detail)

    ignored = exclude_pids or set()
    markers = (
        "server/wifi_ble_service.py",
        "tests/helloworld/server_link_test.py",
    )
    hits: list[str] = []
    for line in out.splitlines():
        item = line.strip()
        if not item:
            continue

        parts = item.split(maxsplit=1)
        if len(parts) < 2:
            continue
        try:
            pid = int(parts[0])
        except ValueError:
            continue
        if pid in ignored:
            continue

        cmdline = parts[1]
        if "python" not in cmdline:
            continue
        if any(marker in cmdline for marker in markers):
            hits.append(f"{pid} {cmdline}")

    if not hits:
        return CheckResult("residual_server_process", True, "No known BLE server process residue")

    sample = " | ".join(hits[:2])
    return CheckResult(
        "residual_server_process",
        False,
        f"Potential residue process detected: {sample}",
    )


def run_preflight_checks(adapter: str) -> PreflightReport:
    current_pid = os.getpid()
    parent_pid = os.getppid()
    checks = [
        _check_adapter_exists(adapter),
        _check_adapter_state(adapter),
        _check_bluez_active(),
        _check_advertising_capacity(),
        _check_residual_processes(exclude_pids={current_pid, parent_pid}),
    ]
    return PreflightReport(checks=checks)


def format_preflight_report(report: PreflightReport) -> str:
    lines = ["[preflight] BLE advertising checks"]
    for item in report.checks:
        status = "PASS" if item.ok else "FAIL"
        lines.append(f"[preflight] {status:<4} {item.name}: {item.detail}")
    lines.append(f"[preflight] overall: {'PASS' if report.ok else 'FAIL'}")
    return "\n".join(lines)
