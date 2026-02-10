from __future__ import annotations

import time
from enum import Enum
from typing import Any

from client.gui.state import GuiState, TaskResult
from client.gui.view import (
    KEY_RESULT_OVERVIEW,
    KEY_RESULT_RAW,
    KEY_RESULT_TABS,
    KEY_STATUS_SUMMARY,
    KEY_STATUS_TABLE,
    KEY_TAB_OVERVIEW,
    KEY_TAB_RAW,
    KEY_TAB_STATUS,
    KEY_TAB_WIFI,
    KEY_WIFI_SUMMARY,
    KEY_WIFI_TABLE,
)
from client.models import ResultCode
from protocol.command_ids import CMD_STATUS, CMD_WIFI_SCAN


class ResultTab(str, Enum):
    OVERVIEW = "overview"
    STATUS = "status"
    WIFI = "wifi"
    RAW = "raw"


_TAB_KEY_BY_RESULT_TAB: dict[ResultTab, str] = {
    ResultTab.OVERVIEW: KEY_TAB_OVERVIEW,
    ResultTab.STATUS: KEY_TAB_STATUS,
    ResultTab.WIFI: KEY_TAB_WIFI,
    ResultTab.RAW: KEY_TAB_RAW,
}


def _signal_bar(signal: int) -> str:
    clamped = max(0, min(signal, 100))
    steps = int(round(clamped / 20.0))
    return f"{'█' * steps}{'░' * (5 - steps)}"


def _status_row_style(value: str) -> tuple[str, str]:
    lowered = value.lower()
    if any(token in lowered for token in ("connected", "active", "enabled", "ok", "success")):
        return "#14532D", "#DCFCE7"
    if any(token in lowered for token in ("failed", "error", "timeout", "unknown (", "ble ")):
        return "#7F1D1D", "#FEE2E2"
    if any(token in lowered for token in ("disconnected", "unknown", "no wifi", "no wifi device", "service not found")):
        return "#92400E", "#FEF3C7"
    return "#1F2937", "#F3F4F6"


def _wifi_row_style(signal: int) -> tuple[str, str]:
    if signal >= 75:
        return "#14532D", "#DCFCE7"
    if signal >= 45:
        return "#92400E", "#FEF3C7"
    return "#7F1D1D", "#FEE2E2"


class ResultPanelPresenter:
    def __init__(self, window: Any) -> None:
        self._window = window

    def initialize(self, state: GuiState) -> None:
        self._render_status_tab(state)
        self._render_wifi_tab(state)
        self._render_raw_tab(state)

    def preferred_tab_for_command(self, command: str | None) -> ResultTab:
        if command == CMD_STATUS:
            return ResultTab.STATUS
        if command == CMD_WIFI_SCAN:
            return ResultTab.WIFI
        if command:
            return ResultTab.RAW
        return ResultTab.OVERVIEW

    def render_result(
        self,
        state: GuiState,
        result: TaskResult,
        *,
        command: str | None = None,
        preferred_tab: ResultTab = ResultTab.OVERVIEW,
    ) -> None:
        state.last_result = result
        state.last_result_command = command
        payload = result.payload if isinstance(result.payload, dict) else {}
        state.last_raw_text = str(payload.get("raw_text", result.message) or result.message)

        if command == CMD_STATUS:
            status_rows = payload.get("status_rows")
            state.last_status_rows = status_rows if isinstance(status_rows, list) else []
        elif command == CMD_WIFI_SCAN:
            wifi_rows = payload.get("wifi_scan_rows")
            agg_rows = payload.get("wifi_scan_aggregated_rows")
            state.last_wifi_rows = wifi_rows if isinstance(wifi_rows, list) else []
            state.last_wifi_aggregated_rows = agg_rows if isinstance(agg_rows, list) else []

        code_text = result.code.name if isinstance(result.code, ResultCode) else "UNKNOWN"
        color = "#065F46" if result.ok else "#B91C1C"
        lines = [f"[{code_text}] {result.message}", ""]
        if command:
            lines.append(f"命令: {command}    时间: {time.strftime('%H:%M:%S')}")
        else:
            lines.append(f"时间: {time.strftime('%H:%M:%S')}")
        ip = payload.get("ip")
        ssh_user = payload.get("ssh_user")
        if isinstance(ip, str) and ip:
            lines.append(f"Server IP: {ip}")
            if isinstance(ssh_user, str) and ssh_user:
                lines.append(f"SSH: ssh {ssh_user}@{ip}")
        if command == CMD_STATUS and not state.last_status_rows:
            lines.append("状态结构化解析失败，已在“原始”页展示完整输出。")
        if command == CMD_WIFI_SCAN and not state.last_wifi_aggregated_rows:
            lines.append("Wi-Fi 扫描未返回可展示的 SSID。")

        self._window[KEY_RESULT_OVERVIEW].update(value="\n".join(lines), text_color_for_value=color)
        self._render_status_tab(state)
        self._render_wifi_tab(state)
        self._render_raw_tab(state)
        self.switch_tab(preferred_tab)

    def switch_tab(self, tab: ResultTab) -> None:
        tab_key = _TAB_KEY_BY_RESULT_TAB[tab]
        try:
            self._window[KEY_RESULT_TABS].Widget.select(self._window[tab_key].Widget)
        except Exception:
            return

    def _render_status_tab(self, state: GuiState) -> None:
        rows = [[row.get("key", "-"), row.get("value", "-")] for row in state.last_status_rows]
        row_colors: list[tuple[int, str, str]] = []
        has_warn = False
        has_error = False
        for idx, row in enumerate(state.last_status_rows):
            value = str(row.get("value", "-"))
            fg, bg = _status_row_style(value)
            row_colors.append((idx, fg, bg))
            lowered = value.lower()
            if any(token in lowered for token in ("failed", "error", "timeout")):
                has_error = True
            elif any(token in lowered for token in ("disconnected", "unknown", "no wifi", "service not found")):
                has_warn = True

        if not rows:
            self._window[KEY_STATUS_SUMMARY].update(value="暂无状态结构化数据", text_color="#6B7280")
            rows = [["-", "暂无状态结构化数据"]]
            row_colors = []
        elif has_error:
            self._window[KEY_STATUS_SUMMARY].update(value="状态概览：异常", text_color="#B91C1C")
        elif has_warn:
            self._window[KEY_STATUS_SUMMARY].update(value="状态概览：部分异常", text_color="#B45309")
        else:
            self._window[KEY_STATUS_SUMMARY].update(value="状态概览：健康", text_color="#166534")

        try:
            self._window[KEY_STATUS_TABLE].update(values=rows, row_colors=row_colors)
        except Exception:
            self._window[KEY_STATUS_TABLE].update(values=rows)

    def _render_wifi_tab(self, state: GuiState) -> None:
        rows = state.last_wifi_aggregated_rows
        if not rows:
            self._window[KEY_WIFI_SUMMARY].update(value="暂无 Wi-Fi 扫描数据", text_color="#6B7280")
            self._window[KEY_WIFI_TABLE].update(values=[])
            return

        strongest = max(int(item.get("signal", 0) or 0) for item in rows)
        if strongest >= 75:
            summary_color = "#166534"
            quality = "信号优"
        elif strongest >= 45:
            summary_color = "#B45309"
            quality = "信号中"
        else:
            summary_color = "#B91C1C"
            quality = "信号弱"
        summary = f"共 {len(rows)} 个 SSID（聚合后） | 最强信号 {strongest}%"
        self._window[KEY_WIFI_SUMMARY].update(value=f"{summary} | {quality}", text_color=summary_color)

        row_colors: list[tuple[int, str, str]] = []
        table_rows: list[list[str]] = []
        for idx, item in enumerate(rows):
            signal = int(item.get("signal", 0) or 0)
            fg, bg = _wifi_row_style(signal)
            row_colors.append((idx, fg, bg))
            table_rows.append(
                [
                    str(item.get("ssid", "")),
                    f"{_signal_bar(signal)} {signal:>3d}%",
                    str(item.get("chan", "-")),
                ]
            )

        try:
            self._window[KEY_WIFI_TABLE].update(values=table_rows, row_colors=row_colors)
        except Exception:
            self._window[KEY_WIFI_TABLE].update(values=table_rows)

    def _render_raw_tab(self, state: GuiState) -> None:
        self._window[KEY_RESULT_RAW].update(value=state.last_raw_text)
