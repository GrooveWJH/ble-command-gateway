#!/usr/bin/env python3
"""Legacy UART demo entrypoint.

This script is deprecated for core provisioning development.
Use `app/server_main.py` for the production command protocol server.
"""

from __future__ import annotations

import argparse
import runpy
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Deprecated legacy UART demo launcher")
    parser.add_argument(
        "--run-legacy",
        action="store_true",
        help="Acknowledge and run the legacy demo implementation",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.run_legacy:
        print("[deprecated] Refusing to run legacy demo by default.")
        print("[deprecated] Use app/server_main.py for core development.")
        print("[deprecated] If needed, rerun with: scripts/bless_uart.py --run-legacy")
        return 2
    legacy = Path(__file__).resolve().parents[1] / "tools" / "legacy" / "bless_uart_demo.py"
    print("[deprecated] scripts/bless_uart.py is legacy demo. Running tools/legacy/bless_uart_demo.py")
    runpy.run_path(str(legacy), run_name="__main__")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
