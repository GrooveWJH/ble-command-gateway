from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Literal

from client.library_models import DeviceInfo
from client.models import ResultCode


@dataclass(frozen=True)
class TaskResult:
    ok: bool
    code: ResultCode | None
    message: str
    payload: dict[str, Any] = field(default_factory=dict)


@dataclass
class GuiState:
    target_name: str
    scan_timeout: int
    wait_timeout: int
    verbose: bool
    ssid: str = ""
    password: str = ""
    selected_device: DeviceInfo | None = None
    session_connected: bool = False
    busy: bool = False
    busy_task: str = ""
    scan_run_id: int = 0
    scan_mode: Literal["idle", "scanning", "stopping", "connecting"] = "idle"
    scan_total_devices: int = 0
    scan_matched_devices: int = 0
    scan_summary: str = "未扫描"
    pending_connect_device: DeviceInfo | None = None
    heartbeat_failures: int = 0
    heartbeat_inflight: bool = False
    heartbeat_next_due_at: float = 0.0
    last_result_command: str | None = None
    last_status_rows: list[dict[str, str]] = field(default_factory=list)
    last_wifi_rows: list[dict[str, object]] = field(default_factory=list)
    last_wifi_aggregated_rows: list[dict[str, object]] = field(default_factory=list)
    last_raw_text: str = ""
    last_result: TaskResult | None = None
    display_devices: list[DeviceInfo] = field(default_factory=list)

    def has_selected_device(self) -> bool:
        return self.selected_device is not None

    def has_wifi_ssid(self) -> bool:
        return bool(self.ssid.strip())

    def can_scan(self) -> bool:
        return not self.busy

    def can_connect(self) -> bool:
        if not self.has_selected_device():
            return False
        if not self.busy:
            return True
        return self.busy_task == "scan" and self.scan_mode in {"scanning", "stopping"}

    def can_disconnect(self) -> bool:
        return (not self.busy) and self.session_connected

    def can_provision(self) -> bool:
        return (not self.busy) and self.session_connected and self.has_wifi_ssid()

    def can_run_diagnostic(self) -> bool:
        return (not self.busy) and self.session_connected

    def reset_heartbeat(self) -> None:
        self.heartbeat_failures = 0
        self.heartbeat_inflight = False
        self.heartbeat_next_due_at = 0.0

    def arm_heartbeat(self, now: float, *, interval_sec: float) -> None:
        self.heartbeat_failures = 0
        self.heartbeat_inflight = False
        self.heartbeat_next_due_at = max(0.0, now + interval_sec)

    def heartbeat_due(self, now: float) -> bool:
        if not self.session_connected or self.busy or self.heartbeat_inflight:
            return False
        if self.heartbeat_next_due_at <= 0.0:
            return False
        return now >= self.heartbeat_next_due_at

    def mark_heartbeat_submitted(self) -> None:
        self.heartbeat_inflight = True

    def mark_heartbeat_success(self, now: float, *, interval_sec: float) -> None:
        self.heartbeat_failures = 0
        self.heartbeat_inflight = False
        self.heartbeat_next_due_at = max(0.0, now + interval_sec)

    def mark_heartbeat_failure(self, now: float, *, interval_sec: float, fail_limit: int) -> bool:
        self.heartbeat_failures += 1
        self.heartbeat_inflight = False
        self.heartbeat_next_due_at = max(0.0, now + interval_sec)
        return self.heartbeat_failures >= fail_limit

    def status_text(self) -> str:
        device_text = self.selected_device.label if self.selected_device else "未选择设备"
        session_text = "已连接" if self.session_connected else "未连接"
        if self.scan_mode == "scanning":
            return (
                f"[扫描中] 总设备 {self.scan_total_devices} | 匹配 {self.scan_matched_devices} | "
                f"{session_text} | {device_text}"
            )
        if self.scan_mode == "stopping":
            return f"[停止扫描] 准备连接 | {session_text} | {device_text}"
        if self.scan_mode == "connecting":
            return f"[连接中] {session_text} | {device_text}"
        if self.busy:
            return f"[忙碌] {self.busy_task} | {session_text} | {device_text}"
        return f"[就绪] {session_text} | {device_text}"
