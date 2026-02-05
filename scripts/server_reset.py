#!/usr/bin/env python3
"""Reset BLE server state for this project on Linux hosts."""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[1]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from server.preflight import format_preflight_report, run_preflight_checks  # noqa: E402

DEFAULT_PATTERNS = [
    "tests/helloworld/server_link_test.py",
    "server/wifi_ble_service.py",
]


def _format_cmd(cmd: list[str]) -> str:
    return " ".join(shlex.quote(part) for part in cmd)


def _run(cmd: list[str], *, dry_run: bool = False) -> int:
    print(f"[reset] $ {_format_cmd(cmd)}")
    if dry_run:
        print("[reset] dry-run")
        return 0

    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if proc.stdout.strip():
        print(proc.stdout.strip())
    if proc.stderr.strip():
        print(proc.stderr.strip())
    print(f"[reset] rc={proc.returncode}")
    return proc.returncode


def _kill_residual_processes(patterns: list[str], dry_run: bool) -> None:
    print("[reset] step: kill residual BLE server processes")
    for pattern in patterns:
        if dry_run:
            _run(["pkill", "-f", pattern], dry_run=True)
            print(f"[reset] planned: {pattern}")
            continue

        rc = _run(["pkill", "-f", pattern], dry_run=dry_run)
        if rc == 0:
            print(f"[reset] killed: {pattern}")
        elif rc == 1:
            print(f"[reset] no match: {pattern}")
        else:
            print(f"[reset] pkill failed: pattern={pattern} rc={rc}")


def _reset_bluetooth(adapter: str, dry_run: bool) -> None:
    print("[reset] step: restart bluetooth service and reset adapter")
    _run(["systemctl", "restart", "bluetooth"], dry_run=dry_run)
    _run(["rfkill", "unblock", "bluetooth"], dry_run=dry_run)
    _run(["hciconfig", adapter, "down"], dry_run=dry_run)

    up_rc = _run(["hciconfig", adapter, "up"], dry_run=dry_run)
    if up_rc == 0:
        return

    print("[reset] hciconfig up failed, fallback to btmgmt power cycle")
    _run(["btmgmt", "-i", adapter, "power", "off"], dry_run=dry_run)
    _run(["btmgmt", "-i", adapter, "power", "on"], dry_run=dry_run)


def _show_status(adapter: str, dry_run: bool) -> None:
    print("[reset] step: show bluetooth status")
    _run(["bluetoothctl", "show"], dry_run=dry_run)
    _run(["hciconfig", "-a", adapter], dry_run=dry_run)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Reset BLE server runtime state")
    parser.add_argument("--adapter", default="hci0", help="Bluetooth adapter name, e.g. hci0")
    parser.add_argument(
        "--pattern",
        action="append",
        default=[],
        help="Additional pkill -f match pattern (repeatable)",
    )
    parser.add_argument("--skip-kill", action="store_true", help="Skip process cleanup")
    parser.add_argument("--skip-status", action="store_true", help="Skip bluetoothctl/hciconfig status output")
    parser.add_argument("--dry-run", action="store_true", help="Print commands without executing")
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if os.geteuid() != 0 and not args.dry_run:
        print("[reset] warning: run with sudo for full effect")

    patterns = [*DEFAULT_PATTERNS, *args.pattern]
    if not args.skip_kill:
        _kill_residual_processes(patterns, args.dry_run)
    _reset_bluetooth(args.adapter, args.dry_run)
    if not args.skip_status:
        _show_status(args.adapter, args.dry_run)

    if args.dry_run:
        return 0

    print("[reset] post-reset preflight")
    report = run_preflight_checks(args.adapter)
    print(format_preflight_report(report))
    return 0 if report.ok else 2


if __name__ == "__main__":
    raise SystemExit(main())
