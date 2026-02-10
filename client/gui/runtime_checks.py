from __future__ import annotations

import sys


def _tk_install_hint() -> str:
    major = sys.version_info.major
    minor = sys.version_info.minor
    return (
        "缺少 tkinter/_tkinter（GUI 运行时依赖）。\n"
        "当前 Python 无法加载 Tk。\n\n"
        "建议修复步骤（macOS + Homebrew Python）：\n"
        f"1) brew install python-tk@{major}.{minor}\n"
        "2) 重新创建虚拟环境并安装依赖：\n"
        "   uv venv --python 3.11\n"
        "   uv sync --group client --group gui\n"
        "3) 使用虚拟环境运行：\n"
        "   uv run python app/client_gui_main.py\n"
    )


def ensure_gui_runtime() -> None:
    try:
        import tkinter  # noqa: F401
    except Exception as exc:  # noqa: BLE001
        raise RuntimeError(_tk_install_hint()) from exc

