from __future__ import annotations

import threading
import time
from concurrent.futures import Future, ThreadPoolExecutor
from typing import Any, Callable

from client.credential_store import save_wifi_credentials
from client.gui.actions import (
    close_session,
    connect_selected,
    make_device_table_rows,
    provision_current,
    run_diagnostic,
    run_heartbeat,
    scan_devices,
)
from client.gui.reporting import LogBuffer
from client.gui.result_panel import ResultPanelPresenter, ResultTab
from client.gui.state import GuiState, TaskResult
from client.gui.task_protocol import GuiTaskDone, GuiTaskKind, GuiTaskRequest
from client.gui.view import (
    DEFAULT_UI_SCALE,
    EVENT_LOG,
    EVENT_SCAN_PROGRESS,
    EVENT_TASK_DONE,
    EVENT_ZOOM,
    KEY_CONNECT,
    KEY_DEVICE_TABLE,
    KEY_DIAG_HELP,
    KEY_DIAG_PING,
    KEY_DIAG_STATUS,
    KEY_DIAG_WIFI_SCAN,
    KEY_DISCONNECT,
    KEY_LOG,
    KEY_LOG_CLEAR,
    KEY_PASSWORD,
    KEY_PROVISION,
    KEY_SAVE_WIFI,
    KEY_SCAN,
    KEY_SCAN_SUMMARY,
    KEY_SCAN_TIMEOUT,
    KEY_SSID,
    KEY_STATUS_BAR,
    KEY_TARGET_NAME,
    KEY_VERBOSE,
    KEY_WAIT_TIMEOUT,
    KEY_ZOOM_IN,
    KEY_ZOOM_OUT,
    KEY_ZOOM_RESET,
    UI_SCALE_STEP,
    apply_ui_scale,
    bind_zoom_shortcuts,
    fit_window_to_scale,
    get_base_tk_scaling,
    update_control_states,
)
from client.library_api import SyncBleGatewayClient, SyncSessionHandle
from client.library_models import DeviceInfo
from client.models import ResultCode
from config.defaults import DEFAULT_SCAN_TIMEOUT, DEFAULT_WAIT_TIMEOUT
from protocol.command_ids import CMD_HELP, CMD_PING, CMD_STATUS, CMD_WIFI_SCAN

try:
    import FreeSimpleGUI as sg
except Exception:  # pragma: no cover - runtime guarded elsewhere
    sg = None
WINDOW_CLOSED_EVENT = sg.WINDOW_CLOSED if sg is not None else "__WINDOW_CLOSED__"

HEARTBEAT_INTERVAL_SEC = 5.0
HEARTBEAT_TIMEOUT_SEC = 5
HEARTBEAT_FAIL_LIMIT = 2


class GuiController:
    def __init__(
        self,
        *,
        window: Any,
        state: GuiState,
        gateway: SyncBleGatewayClient,
        executor: ThreadPoolExecutor,
        log_buffer: LogBuffer,
        presenter: ResultPanelPresenter,
    ) -> None:
        self.window = window
        self.state = state
        self.gateway = gateway
        self.executor = executor
        self.log_buffer = log_buffer
        self.presenter = presenter
        self.active_session: SyncSessionHandle | None = None
        self.scan_stop_event: threading.Event | None = None
        self.tk_base_scaling = get_base_tk_scaling(window)
        self.ui_scale = DEFAULT_UI_SCALE
        self.diag_event_map: dict[str, str] = {
            KEY_DIAG_STATUS: CMD_STATUS,
            KEY_DIAG_WIFI_SCAN: CMD_WIFI_SCAN,
            KEY_DIAG_PING: CMD_PING,
            KEY_DIAG_HELP: CMD_HELP,
        }
        self.event_handlers: dict[object, Callable[[dict[str, Any]], None]] = {
            EVENT_LOG: self._on_event_log,
            KEY_LOG_CLEAR: self._on_event_log_clear,
            EVENT_SCAN_PROGRESS: self._on_event_scan_progress,
            EVENT_ZOOM: self._on_event_zoom,
            KEY_ZOOM_IN: self._on_event_zoom_in,
            KEY_ZOOM_OUT: self._on_event_zoom_out,
            KEY_ZOOM_RESET: self._on_event_zoom_reset,
            EVENT_TASK_DONE: self._on_event_task_done,
            KEY_DEVICE_TABLE: self._on_event_device_table,
            KEY_SAVE_WIFI: self._on_event_save_wifi,
            KEY_SCAN: self._on_event_scan,
            KEY_CONNECT: self._on_event_connect,
            KEY_DISCONNECT: self._on_event_disconnect,
            KEY_PROVISION: self._on_event_provision,
        }

    def initialize(self) -> None:
        self.ui_scale = apply_ui_scale(
            self.window,
            base_scaling=self.tk_base_scaling,
            scale_factor=DEFAULT_UI_SCALE,
        )
        fit_window_to_scale(self.window, self.ui_scale)
        bind_zoom_shortcuts(self.window)
        self.window[KEY_TARGET_NAME].update(value=self.state.target_name)
        self.window[KEY_SCAN_TIMEOUT].update(value=str(self.state.scan_timeout))
        self.window[KEY_WAIT_TIMEOUT].update(value=str(self.state.wait_timeout))
        self.window[KEY_VERBOSE].update(value=self.state.verbose)
        self.window[KEY_SSID].update(value=self.state.ssid)
        self.window[KEY_PASSWORD].update(value=self.state.password)
        update_control_states(self.window, self.state)
        self._render_scan_ui(remaining=None)
        self.presenter.initialize(self.state)
        if self.state.ssid:
            self._append_log(f"已加载缓存 Wi-Fi 凭据: SSID={self.state.ssid}")

    def _mark_connected(self, now: float) -> None:
        self.state.session_connected = True
        self.state.arm_heartbeat(now, interval_sec=HEARTBEAT_INTERVAL_SEC)

    def _mark_disconnected(self) -> None:
        self.state.session_connected = False
        self.state.reset_heartbeat()

    def _append_log(self, message: str) -> None:
        self.log_buffer.append(message)
        self.window[KEY_LOG].update(value=self.log_buffer.render())

    def _safe_int(self, raw: object, *, default: int, name: str) -> tuple[int | None, str | None]:
        text = str(raw or "").strip()
        if not text:
            return default, None
        try:
            value = int(text)
        except ValueError:
            return None, f"{name} 必须是整数"
        if value <= 0:
            return None, f"{name} 必须大于 0"
        return value, None

    def _refresh_from_values(self, values: dict[str, Any]) -> TaskResult | None:
        target_name = str(values.get(KEY_TARGET_NAME, "")).strip()
        if not target_name:
            return TaskResult(ok=False, code=ResultCode.INPUT_ERROR, message="设备名过滤不能为空")

        scan_timeout, scan_err = self._safe_int(
            values.get(KEY_SCAN_TIMEOUT),
            default=DEFAULT_SCAN_TIMEOUT,
            name="扫描超时",
        )
        if scan_err:
            return TaskResult(ok=False, code=ResultCode.INPUT_ERROR, message=scan_err)

        wait_timeout, wait_err = self._safe_int(
            values.get(KEY_WAIT_TIMEOUT),
            default=DEFAULT_WAIT_TIMEOUT,
            name="等待超时",
        )
        if wait_err:
            return TaskResult(ok=False, code=ResultCode.INPUT_ERROR, message=wait_err)

        assert scan_timeout is not None
        assert wait_timeout is not None
        self.state.target_name = target_name
        self.state.scan_timeout = scan_timeout
        self.state.wait_timeout = wait_timeout
        self.state.verbose = bool(values.get(KEY_VERBOSE))
        self.state.ssid = str(values.get(KEY_SSID, ""))
        self.state.password = str(values.get(KEY_PASSWORD, ""))

        self.window[KEY_TARGET_NAME].update(value=self.state.target_name)
        self.window[KEY_SCAN_TIMEOUT].update(value=str(self.state.scan_timeout))
        self.window[KEY_WAIT_TIMEOUT].update(value=str(self.state.wait_timeout))
        return None

    def _device_sort_key(self, device: DeviceInfo) -> tuple[str, str]:
        display_name = (device.adv_name or device.name or "").lower()
        return display_name, device.address

    def _upsert_named_device(self, device: DeviceInfo) -> bool:
        if not (device.adv_name or device.name):
            return False
        for idx, existing in enumerate(self.state.display_devices):
            if existing.address == device.address:
                self.state.display_devices[idx] = device
                self.state.display_devices.sort(key=self._device_sort_key)
                return True
        self.state.display_devices.append(device)
        self.state.display_devices.sort(key=self._device_sort_key)
        return True

    def _refresh_device_table(self) -> None:
        table_rows = make_device_table_rows(self.state.display_devices)
        selected_rows: list[int] = []
        if self.state.selected_device is not None:
            for idx, device in enumerate(self.state.display_devices):
                if device.address == self.state.selected_device.address:
                    selected_rows = [idx]
                    break
        self.window[KEY_DEVICE_TABLE].update(values=table_rows, select_rows=selected_rows)

    def _render_scan_ui(self, *, remaining: int | None) -> None:
        if self.state.scan_mode == "scanning":
            button_text = f"扫描中({remaining}s)" if remaining is not None else "扫描中..."
            self.state.scan_summary = (
                f"扫描中 | 总设备{self.state.scan_total_devices} | 匹配{self.state.scan_matched_devices}"
            )
        elif self.state.scan_mode == "stopping":
            button_text = "停止扫描..."
            self.state.scan_summary = "停止扫描中，准备连接..."
        elif self.state.scan_mode == "connecting":
            button_text = "连接中..."
            if not self.state.scan_summary:
                self.state.scan_summary = "连接中..."
        else:
            button_text = "扫描"
            if not self.state.scan_summary:
                self.state.scan_summary = "未扫描"

        self.window[KEY_SCAN].update(text=button_text)
        self.window[KEY_SCAN_SUMMARY].update(value=self.state.scan_summary)

    def _scan_task(self, *, run_id: int, target_name: str, timeout: int, stop_event: threading.Event) -> TaskResult:
        previous_session = self.active_session
        if previous_session is not None:
            self._close_session_non_blocking(previous_session, timeout_sec=2.0)

        def _emit(payload: dict[str, object]) -> None:
            self.window.write_event_value(EVENT_SCAN_PROGRESS, {"run_id": run_id, **payload})

        def _on_detect(device: Any) -> None:
            mapped = device if isinstance(device, DeviceInfo) else DeviceInfo.from_any(device)
            _emit({"kind": "detect", "device": mapped})

        def _on_progress(elapsed: float, total: float, total_devices: int, matched_devices: int) -> None:
            _emit(
                {
                    "kind": "progress",
                    "elapsed": elapsed,
                    "total": total,
                    "total_devices": total_devices,
                    "matched_devices": matched_devices,
                }
            )

        return scan_devices(
            self.gateway,
            target_name=target_name,
            timeout=timeout,
            on_progress=_on_progress,
            on_detect=_on_detect,
            stop_event=stop_event,
        )

    def _close_session_non_blocking(self, session: SyncSessionHandle, *, timeout_sec: float) -> None:
        done = threading.Event()
        outcome: dict[str, TaskResult] = {}

        def _worker() -> None:
            try:
                outcome["result"] = close_session(session)
            finally:
                done.set()

        thread = threading.Thread(target=_worker, daemon=True)
        thread.start()
        done.wait(timeout=timeout_sec)

    def _submit_task(
        self,
        request: GuiTaskRequest,
        func: Callable[..., TaskResult],
        *task_args: object,
        **task_kwargs: object,
    ) -> bool:
        if self.state.busy:
            return False
        if request.kind is GuiTaskKind.HEARTBEAT and self.state.heartbeat_inflight:
            return False

        if request.affects_busy:
            self.state.busy = True
            self.state.busy_task = request.kind.value
        elif request.kind is GuiTaskKind.HEARTBEAT:
            self.state.mark_heartbeat_submitted()
        update_control_states(self.window, self.state)

        future = self.executor.submit(func, *task_args, **task_kwargs)

        def _done(done_future: Future[TaskResult]) -> None:
            self.window.write_event_value(
                EVENT_TASK_DONE,
                GuiTaskDone(request=request, future=done_future),
            )

        future.add_done_callback(_done)
        return True

    def _handle_task_done_event(self, payload: object) -> None:
        if not isinstance(payload, GuiTaskDone):
            return
        done = payload
        if done.kind is GuiTaskKind.SCAN:
            run_id = int(done.meta.get("run_id", -1))
            if run_id != self.state.scan_run_id:
                return

        if done.affects_busy:
            self.state.busy = False
            self.state.busy_task = ""

        result: TaskResult
        try:
            result = done.future.result()
        except Exception as exc:  # noqa: BLE001
            result = TaskResult(
                ok=False,
                code=ResultCode.FAILED,
                message=f"后台任务失败: {type(exc).__name__}: {exc}",
            )

        if done.kind is GuiTaskKind.HEARTBEAT:
            self._handle_done_heartbeat(result)
            return

        command: str | None = None
        preferred_tab = ResultTab.OVERVIEW
        connect_target: DeviceInfo | None = None

        if done.kind is GuiTaskKind.SCAN:
            connect_target = self._handle_done_scan(result)
        elif done.kind is GuiTaskKind.CONNECT:
            self._handle_done_connect(result)
        elif done.kind is GuiTaskKind.DISCONNECT:
            self._handle_done_disconnect(result)
        elif done.kind is GuiTaskKind.PROVISION:
            self._handle_done_provision(result)
        elif done.kind is GuiTaskKind.DIAGNOSTIC:
            command, preferred_tab = self._handle_done_diagnostic(result)

        if done.kind in {GuiTaskKind.SCAN, GuiTaskKind.CONNECT, GuiTaskKind.DISCONNECT}:
            self._render_scan_ui(remaining=None)

        self.presenter.render_result(
            self.state,
            result,
            command=command,
            preferred_tab=preferred_tab,
        )
        update_control_states(self.window, self.state)

        if done.kind is GuiTaskKind.SCAN and connect_target is not None:
            self.state.selected_device = connect_target
            self.state.scan_mode = "connecting"
            self._render_scan_ui(remaining=None)
            update_control_states(self.window, self.state)
            self._submit_task(
                GuiTaskRequest(kind=GuiTaskKind.CONNECT),
                connect_selected,
                self.gateway,
                connect_target,
            )

    def _handle_done_heartbeat(self, result: TaskResult) -> None:
        if self.active_session is None or not self.state.session_connected:
            self.state.reset_heartbeat()
            update_control_states(self.window, self.state)
            return

        if result.ok:
            self.state.mark_heartbeat_success(time.monotonic(), interval_sec=HEARTBEAT_INTERVAL_SEC)
            update_control_states(self.window, self.state)
            return

        should_disconnect = self.state.mark_heartbeat_failure(
            time.monotonic(),
            interval_sec=HEARTBEAT_INTERVAL_SEC,
            fail_limit=HEARTBEAT_FAIL_LIMIT,
        )
        if not should_disconnect:
            update_control_states(self.window, self.state)
            return

        close_result = close_session(self.active_session)
        self.active_session = None
        self.state.scan_mode = "idle"
        self._mark_disconnected()
        if not close_result.ok:
            self.window[KEY_STATUS_BAR].update(
                value=f"{self.state.status_text()} | 心跳断开后清理失败: {close_result.message}"
            )
            update_control_states(self.window, self.state)
            return

        self.window[KEY_STATUS_BAR].update(value=f"{self.state.status_text()} | 心跳超时，连接已断开")
        update_control_states(self.window, self.state)

    def _handle_done_scan(self, result: TaskResult) -> DeviceInfo | None:
        self.scan_stop_event = None
        self.state.scan_mode = "idle"
        self.active_session = None
        self._mark_disconnected()
        self.state.scan_summary = result.message

        final_devices = result.payload.get("display_devices", [])
        if isinstance(final_devices, list):
            self.state.display_devices = sorted(
                [d for d in final_devices if isinstance(d, DeviceInfo)],
                key=self._device_sort_key,
            )
        if self.state.selected_device is not None:
            found = next(
                (d for d in self.state.display_devices if d.address == self.state.selected_device.address),
                None,
            )
            self.state.selected_device = found
        if self.state.selected_device is None and self.state.display_devices:
            self.state.selected_device = self.state.display_devices[0]

        self._refresh_device_table()
        self._render_scan_ui(remaining=None)

        auto_selected = result.payload.get("auto_selected")
        if auto_selected in self.state.display_devices:
            idx = self.state.display_devices.index(auto_selected)
            self.state.selected_device = auto_selected
            self.window[KEY_DEVICE_TABLE].update(select_rows=[idx])
            self._append_log(f"自动选择设备: {auto_selected.label}")

        connect_target: DeviceInfo | None
        if isinstance(self.state.pending_connect_device, DeviceInfo):
            connect_target = self.state.pending_connect_device
        else:
            auto_connect = result.payload.get("auto_connect_device")
            connect_target = auto_connect if isinstance(auto_connect, DeviceInfo) else None
        self.state.pending_connect_device = None
        return connect_target

    def _handle_done_connect(self, result: TaskResult) -> None:
        self.state.scan_mode = "idle"
        if not result.ok:
            self.active_session = None
            self._mark_disconnected()
            self._append_log(f"连接失败: {result.message}")
            return

        session = result.payload.get("session")
        if not isinstance(session, SyncSessionHandle):
            self.active_session = None
            self._mark_disconnected()
            self._append_log("连接失败: 会话无效")
            return

        self.active_session = session
        if bool(self.active_session.is_connected):
            self._mark_connected(time.monotonic())
        else:
            self._mark_disconnected()

        selected = result.payload.get("device")
        if selected in self.state.display_devices:
            self.state.selected_device = selected
            self._append_log(f"连接成功: {selected.label}")
        else:
            self._append_log("连接成功: 已建立会话")

    def _handle_done_disconnect(self, _result: TaskResult) -> None:
        self.state.scan_mode = "idle"
        self.active_session = None
        self._mark_disconnected()
        self._append_log("会话已断开")

    def _handle_done_provision(self, _result: TaskResult) -> None:
        if self.active_session is not None and bool(self.active_session.is_connected):
            self._mark_connected(time.monotonic())
            return
        self.active_session = None
        self._mark_disconnected()

    def _handle_done_diagnostic(self, result: TaskResult) -> tuple[str | None, ResultTab]:
        if self.active_session is not None and bool(self.active_session.is_connected):
            self._mark_connected(time.monotonic())
        else:
            self.active_session = None
            self._mark_disconnected()

        payload = result.payload if isinstance(result.payload, dict) else {}
        command = payload.get("command")
        command_text = str(command) if isinstance(command, str) else None
        return command_text, self.presenter.preferred_tab_for_command(command_text)

    def _maybe_submit_heartbeat(self, now: float) -> None:
        if self.active_session is None:
            return
        if not self.state.heartbeat_due(now):
            return
        self._submit_task(
            GuiTaskRequest(kind=GuiTaskKind.HEARTBEAT, affects_busy=False),
            run_heartbeat,
            self.active_session,
            timeout=HEARTBEAT_TIMEOUT_SEC,
        )

    def _handle_scan_progress_event(self, payload: object) -> None:
        if not isinstance(payload, dict):
            return
        run_id = int(payload.get("run_id", -1))
        if run_id != self.state.scan_run_id:
            return

        kind = str(payload.get("kind", ""))
        if kind == "detect":
            device = payload.get("device")
            if isinstance(device, DeviceInfo) and self._upsert_named_device(device):
                if self.state.selected_device is None:
                    self.state.selected_device = device
                self._refresh_device_table()
        else:
            elapsed = payload.get("elapsed")
            total = payload.get("total")
            self.state.scan_total_devices = int(payload.get("total_devices") or 0)
            self.state.scan_matched_devices = int(payload.get("matched_devices") or 0)
            if isinstance(elapsed, (float, int)) and isinstance(total, (float, int)):
                remain = max(int(total - elapsed), 0)
                self._render_scan_ui(remaining=remain)
            else:
                self._render_scan_ui(remaining=None)

        self.window[KEY_STATUS_BAR].update(value=self.state.status_text())

    def _on_event_log(self, values: dict[str, Any]) -> None:
        self.log_buffer.append(values.get(EVENT_LOG))
        self.window[KEY_LOG].update(value=self.log_buffer.render())

    def _on_event_log_clear(self, _values: dict[str, Any]) -> None:
        self.log_buffer.clear()
        self.window[KEY_LOG].update(value="")
        self.window[KEY_STATUS_BAR].update(value=f"{self.state.status_text()} | 日志已清空")

    def _on_event_scan_progress(self, values: dict[str, Any]) -> None:
        self._handle_scan_progress_event(values.get(EVENT_SCAN_PROGRESS))

    def _on_event_zoom(self, values: dict[str, Any]) -> None:
        self._handle_zoom(int(values.get(EVENT_ZOOM) or 0))

    def _on_event_zoom_in(self, _values: dict[str, Any]) -> None:
        self._handle_zoom(+1)

    def _on_event_zoom_out(self, _values: dict[str, Any]) -> None:
        self._handle_zoom(-1)

    def _on_event_zoom_reset(self, _values: dict[str, Any]) -> None:
        self.ui_scale = apply_ui_scale(
            self.window,
            base_scaling=self.tk_base_scaling,
            scale_factor=DEFAULT_UI_SCALE,
        )
        fit_window_to_scale(self.window, self.ui_scale)
        self.window[KEY_STATUS_BAR].update(value=f"{self.state.status_text()} | 缩放 {int(self.ui_scale * 100)}%")

    def _on_event_task_done(self, values: dict[str, Any]) -> None:
        self._handle_task_done_event(values.get(EVENT_TASK_DONE))

    def _on_event_device_table(self, values: dict[str, Any]) -> None:
        self._handle_device_table(values)

    def _on_event_save_wifi(self, values: dict[str, Any]) -> None:
        self._handle_save_wifi(values)

    def _on_event_scan(self, values: dict[str, Any]) -> None:
        self._handle_scan(values)

    def _on_event_connect(self, values: dict[str, Any]) -> None:
        self._handle_connect(values)

    def _on_event_disconnect(self, _values: dict[str, Any]) -> None:
        self._handle_disconnect()

    def _on_event_provision(self, values: dict[str, Any]) -> None:
        self._handle_provision(values)

    def _process_event(self, event: object, values: dict[str, Any]) -> bool:
        if event == WINDOW_CLOSED_EVENT:
            if self.state.busy:
                self.window[KEY_STATUS_BAR].update("任务执行中，无法退出。请等待当前任务结束。")
                return False
            return True

        handler = self.event_handlers.get(event)
        if handler is not None:
            handler(values)
            return False

        if isinstance(event, str) and event in self.diag_event_map:
            self._handle_diagnostic(event, values)
            return False

        return False

    def _handle_zoom(self, delta: int) -> None:
        if delta == 0:
            return
        self.ui_scale = apply_ui_scale(
            self.window,
            base_scaling=self.tk_base_scaling,
            scale_factor=self.ui_scale + (UI_SCALE_STEP * delta),
        )
        fit_window_to_scale(self.window, self.ui_scale)
        self.window[KEY_STATUS_BAR].update(value=f"{self.state.status_text()} | 缩放 {int(self.ui_scale * 100)}%")

    def _handle_device_table(self, values: dict[str, Any]) -> None:
        selected_rows = values.get(KEY_DEVICE_TABLE) or []
        if not selected_rows:
            return
        index = int(selected_rows[0])
        if 0 <= index < len(self.state.display_devices):
            self.state.selected_device = self.state.display_devices[index]
            update_control_states(self.window, self.state)

    def _render_input_error(self, error: TaskResult) -> None:
        self.presenter.render_result(self.state, error)
        update_control_states(self.window, self.state)

    def _handle_save_wifi(self, values: dict[str, Any]) -> None:
        update_err = self._refresh_from_values(values)
        if update_err is not None:
            self._render_input_error(update_err)
            return

        if not self.state.ssid.strip():
            self._render_input_error(TaskResult(ok=False, code=ResultCode.INPUT_ERROR, message="SSID 不能为空"))
            return

        if save_wifi_credentials(self.state.ssid, self.state.password):
            result = TaskResult(ok=True, code=ResultCode.SUCCESS, message=f"已保存 Wi-Fi 凭据: {self.state.ssid}")
        else:
            result = TaskResult(ok=False, code=ResultCode.FAILED, message="保存 Wi-Fi 凭据失败")

        self._append_log(result.message)
        self.presenter.render_result(self.state, result)
        update_control_states(self.window, self.state)

    def _handle_scan(self, values: dict[str, Any]) -> None:
        update_err = self._refresh_from_values(values)
        if update_err is not None:
            self._render_input_error(update_err)
            return

        # Start every scan from a clean list to avoid stale devices from previous runs.
        self.state.display_devices = []
        self.state.selected_device = None
        self._refresh_device_table()

        self.state.scan_run_id += 1
        self.state.scan_mode = "scanning"
        self.state.scan_total_devices = 0
        self.state.scan_matched_devices = 0
        self.state.scan_summary = "扫描中 | 总设备0 | 匹配0"
        self.state.pending_connect_device = None
        self._mark_disconnected()
        self.scan_stop_event = threading.Event()
        self._render_scan_ui(remaining=self.state.scan_timeout)

        accepted = self._submit_task(
            GuiTaskRequest(kind=GuiTaskKind.SCAN, meta={"run_id": self.state.scan_run_id}),
            self._scan_task,
            run_id=self.state.scan_run_id,
            target_name=self.state.target_name,
            timeout=self.state.scan_timeout,
            stop_event=self.scan_stop_event,
        )
        if accepted:
            return

        self.state.scan_mode = "idle"
        self._render_scan_ui(remaining=None)
        self.presenter.render_result(
            self.state,
            TaskResult(ok=False, code=ResultCode.FAILED, message="扫描任务提交失败：当前仍有任务在执行"),
        )
        update_control_states(self.window, self.state)

    def _handle_connect(self, values: dict[str, Any]) -> None:
        update_err = self._refresh_from_values(values)
        if update_err is not None:
            self._render_input_error(update_err)
            return

        if self.state.selected_device is None:
            self._render_input_error(TaskResult(ok=False, code=ResultCode.NOT_FOUND, message="请先选择设备"))
            return

        if self.state.busy and self.state.busy_task == GuiTaskKind.SCAN.value:
            self.state.pending_connect_device = self.state.selected_device
            self.state.scan_mode = "stopping"
            self._render_scan_ui(remaining=None)
            if self.scan_stop_event is not None:
                self.scan_stop_event.set()
            update_control_states(self.window, self.state)
            self.window[KEY_STATUS_BAR].update(value=self.state.status_text())
            return

        self.state.scan_mode = "connecting"
        self._render_scan_ui(remaining=None)
        self._submit_task(
            GuiTaskRequest(kind=GuiTaskKind.CONNECT),
            connect_selected,
            self.gateway,
            self.state.selected_device,
        )

    def _handle_disconnect(self) -> None:
        session = self.active_session
        self.active_session = None
        self.state.scan_mode = "idle"
        self._mark_disconnected()
        self.state.busy = False
        self.state.busy_task = ""
        self._render_scan_ui(remaining=None)
        self.window[KEY_STATUS_BAR].update(value=f"{self.state.status_text()} | 正在断开...")
        update_control_states(self.window, self.state)

        if session is None:
            self._append_log("会话已断开")
            return

        self._submit_task(
            GuiTaskRequest(kind=GuiTaskKind.DISCONNECT, affects_busy=False),
            close_session,
            session,
        )

    def _handle_provision(self, values: dict[str, Any]) -> None:
        update_err = self._refresh_from_values(values)
        if update_err is not None:
            self._render_input_error(update_err)
            return

        if self.active_session is None or not self.state.session_connected:
            self._render_input_error(TaskResult(ok=False, code=ResultCode.NOT_FOUND, message="请先连接设备"))
            return

        self._submit_task(
            GuiTaskRequest(kind=GuiTaskKind.PROVISION),
            provision_current,
            self.active_session,
            ssid=self.state.ssid,
            password=self.state.password,
            timeout=self.state.wait_timeout,
            verbose=self.state.verbose,
        )

    def _handle_diagnostic(self, event: str, values: dict[str, Any]) -> None:
        update_err = self._refresh_from_values(values)
        if update_err is not None:
            self._render_input_error(update_err)
            return

        if self.active_session is None or not self.state.session_connected:
            self._render_input_error(TaskResult(ok=False, code=ResultCode.NOT_FOUND, message="请先连接设备"))
            return

        command_id = self.diag_event_map[event]
        self._submit_task(
            GuiTaskRequest(kind=GuiTaskKind.DIAGNOSTIC, meta={"command": command_id}),
            run_diagnostic,
            self.active_session,
            command_id=command_id,
            timeout=self.state.wait_timeout,
        )

    def run(self) -> int:
        self.initialize()
        try:
            while True:
                event, values = self.window.read(timeout=120)
                self._maybe_submit_heartbeat(time.monotonic())
                if self._process_event(event, values):
                    break
        finally:
            if self.active_session is not None:
                close_session(self.active_session)
            self.gateway.close()
            self.executor.shutdown(wait=False, cancel_futures=True)
            self.window.close()

        if self.state.last_result and isinstance(self.state.last_result.code, ResultCode):
            return int(self.state.last_result.code)
        return 0
