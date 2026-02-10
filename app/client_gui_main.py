"""GUI client application entrypoint."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[1]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))


def parse_args() -> argparse.Namespace:
    from config.defaults import DEFAULT_DEVICE_NAME, DEFAULT_SCAN_TIMEOUT, DEFAULT_WAIT_TIMEOUT

    parser = argparse.ArgumentParser(
        description="云纵无线配置器 (FreeSimpleGUI)",
    )
    parser.add_argument("--target-name", default=DEFAULT_DEVICE_NAME, help="BLE device name contains this string")
    parser.add_argument("--scan-timeout", type=int, default=DEFAULT_SCAN_TIMEOUT, help="BLE scan timeout seconds")
    parser.add_argument("--wait-timeout", type=int, default=DEFAULT_WAIT_TIMEOUT, help="Wait status timeout seconds")
    parser.add_argument("--verbose", action="store_true", help="Print all status polling logs")
    return parser.parse_args()


def run() -> int:
    args = parse_args()
    try:
        from client.gui.runtime_checks import ensure_gui_runtime  # noqa: E402
    except ModuleNotFoundError as exc:
        if exc.name in {"bleak", "FreeSimpleGUI"}:
            raise RuntimeError(
                f"缺少依赖 {exc.name!r}。请先安装并使用虚拟环境运行：\n"
                "  uv sync --group client --group gui\n"
                "  uv run python app/client_gui_main.py"
            ) from exc
        raise
    except Exception as exc:  # noqa: BLE001
        raise RuntimeError(f"GUI 启动失败: {type(exc).__name__}: {exc}") from exc

    ensure_gui_runtime()
    try:
        from client.gui.app import run_gui  # noqa: E402
    except ModuleNotFoundError as exc:
        if exc.name in {"bleak", "FreeSimpleGUI"}:
            raise RuntimeError(
                f"缺少依赖 {exc.name!r}。请先安装并使用虚拟环境运行：\n"
                "  uv sync --group client --group gui\n"
                "  uv run python app/client_gui_main.py"
            ) from exc
        raise

    return run_gui(args)


if __name__ == "__main__":
    raise SystemExit(run())
