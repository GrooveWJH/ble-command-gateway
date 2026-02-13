from __future__ import annotations

import unittest
from concurrent.futures import Future, ThreadPoolExecutor
from unittest.mock import Mock, patch

from client.gui.controller import GuiController
from client.gui.reporting import LogBuffer
from client.gui.result_panel import ResultPanelPresenter
from client.gui.state import GuiState, TaskResult
from client.gui.task_protocol import GuiTaskDone, GuiTaskKind, GuiTaskRequest
from client.library_models import DeviceInfo
from client.models import ResultCode
from client.gui.view import KEY_PASSWORD, KEY_SCAN_TIMEOUT, KEY_SSID, KEY_TARGET_NAME, KEY_VERBOSE, KEY_WAIT_TIMEOUT


class GuiControllerRoutingTests(unittest.TestCase):
    def _build_controller(self) -> GuiController:
        state = GuiState(target_name="Y", scan_timeout=8, wait_timeout=8, verbose=False)
        window = Mock()
        element = Mock()
        element.update = Mock()
        window.__getitem__ = Mock(return_value=element)
        presenter = Mock(spec=ResultPanelPresenter)
        gateway = Mock()
        executor = ThreadPoolExecutor(max_workers=1)
        self.addCleanup(executor.shutdown, wait=False, cancel_futures=True)
        return GuiController(
            window=window,
            state=state,
            gateway=gateway,
            executor=executor,
            log_buffer=LogBuffer(),
            presenter=presenter,
        )

    def _done(self, kind: GuiTaskKind, *, affects_busy: bool = True) -> GuiTaskDone:
        request = GuiTaskRequest(kind=kind, affects_busy=affects_busy)
        future: Future[TaskResult] = Future()
        future.set_result(TaskResult(ok=True, code=ResultCode.SUCCESS, message="ok", payload={}))
        return GuiTaskDone(request=request, future=future)

    @patch("client.gui.controller.update_control_states")
    def test_task_done_routes_to_scan_handler(self, _update_state: Mock) -> None:
        controller = self._build_controller()
        controller.state.scan_run_id = 3
        controller._handle_done_scan = Mock(return_value=None)  # type: ignore[method-assign]
        controller._render_scan_ui = Mock()  # type: ignore[method-assign]
        request = GuiTaskRequest(kind=GuiTaskKind.SCAN, meta={"run_id": 3})
        future: Future[TaskResult] = Future()
        future.set_result(TaskResult(ok=True, code=ResultCode.SUCCESS, message="ok", payload={}))
        done = GuiTaskDone(request=request, future=future)

        controller._handle_task_done_event(done)

        controller._handle_done_scan.assert_called_once()  # type: ignore[attr-defined]

    @patch("client.gui.controller.update_control_states")
    def test_task_done_routes_to_connect_handler(self, _update_state: Mock) -> None:
        controller = self._build_controller()
        controller._handle_done_connect = Mock()  # type: ignore[method-assign]
        controller._render_scan_ui = Mock()  # type: ignore[method-assign]

        controller._handle_task_done_event(self._done(GuiTaskKind.CONNECT))

        controller._handle_done_connect.assert_called_once()  # type: ignore[attr-defined]

    @patch("client.gui.controller.update_control_states")
    def test_task_done_routes_to_diagnostic_handler(self, _update_state: Mock) -> None:
        controller = self._build_controller()
        controller._handle_done_diagnostic = Mock(return_value=("status", controller.presenter.preferred_tab_for_command("status")))  # type: ignore[method-assign]

        controller._handle_task_done_event(self._done(GuiTaskKind.DIAGNOSTIC))

        controller._handle_done_diagnostic.assert_called_once()  # type: ignore[attr-defined]
        controller.presenter.render_result.assert_called_once()  # type: ignore[attr-defined]

    @patch("client.gui.controller.update_control_states")
    def test_task_done_routes_to_heartbeat_handler_without_render(self, _update_state: Mock) -> None:
        controller = self._build_controller()
        controller._handle_done_heartbeat = Mock()  # type: ignore[method-assign]

        controller._handle_task_done_event(self._done(GuiTaskKind.HEARTBEAT, affects_busy=False))

        controller._handle_done_heartbeat.assert_called_once()  # type: ignore[attr-defined]
        controller.presenter.render_result.assert_not_called()  # type: ignore[attr-defined]

    @patch("client.gui.controller.update_control_states")
    def test_handle_scan_rolls_back_when_submit_rejected(self, _update_state: Mock) -> None:
        controller = self._build_controller()
        controller._submit_task = Mock(return_value=False)  # type: ignore[method-assign]
        controller._render_scan_ui = Mock()  # type: ignore[method-assign]

        values = {
            KEY_TARGET_NAME: "Yundrone",
            KEY_SCAN_TIMEOUT: "25",
            KEY_WAIT_TIMEOUT: "45",
            KEY_VERBOSE: False,
            KEY_SSID: "",
            KEY_PASSWORD: "",
        }
        controller._handle_scan(values)

        self.assertEqual(controller.state.scan_mode, "idle")
        controller.presenter.render_result.assert_called_once()  # type: ignore[attr-defined]

    @patch("client.gui.controller.update_control_states")
    def test_handle_scan_clears_device_list_before_submit(self, _update_state: Mock) -> None:
        controller = self._build_controller()
        controller._submit_task = Mock(return_value=True)  # type: ignore[method-assign]
        controller.state.display_devices = [DeviceInfo(address="AA:BB", name="Old", adv_name="Old")]
        controller.state.selected_device = controller.state.display_devices[0]

        values = {
            KEY_TARGET_NAME: "Yundrone",
            KEY_SCAN_TIMEOUT: "25",
            KEY_WAIT_TIMEOUT: "45",
            KEY_VERBOSE: False,
            KEY_SSID: "",
            KEY_PASSWORD: "",
        }
        controller._handle_scan(values)

        self.assertEqual(controller.state.display_devices, [])
        self.assertIsNone(controller.state.selected_device)

    @patch("client.gui.controller.scan_devices")
    def test_scan_task_closes_existing_session_non_blocking(self, patched_scan_devices: Mock) -> None:
        controller = self._build_controller()
        controller._close_session_non_blocking = Mock()  # type: ignore[method-assign]
        controller.active_session = Mock()
        patched_scan_devices.return_value = TaskResult(ok=True, code=ResultCode.SUCCESS, message="ok", payload={})

        _ = controller._scan_task(
            run_id=1,
            target_name="Yundrone",
            timeout=3,
            stop_event=Mock(),
        )

        controller._close_session_non_blocking.assert_called_once()  # type: ignore[attr-defined]

    @patch("client.gui.controller.update_control_states")
    def test_handle_disconnect_resets_state_immediately(self, _update_state: Mock) -> None:
        controller = self._build_controller()
        controller.active_session = Mock()
        controller.state.session_connected = True
        controller._submit_task = Mock(return_value=True)  # type: ignore[method-assign]
        controller._render_scan_ui = Mock()  # type: ignore[method-assign]

        controller._handle_disconnect()

        self.assertIsNone(controller.active_session)
        self.assertFalse(controller.state.session_connected)
        self.assertFalse(controller.state.busy)
        self.assertEqual(controller.state.busy_task, "")
        controller._submit_task.assert_called_once()  # type: ignore[attr-defined]
        request = controller._submit_task.call_args.args[0]  # type: ignore[attr-defined]
        self.assertEqual(request.kind, GuiTaskKind.DISCONNECT)
        self.assertFalse(request.affects_busy)


if __name__ == "__main__":
    unittest.main()
