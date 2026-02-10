from __future__ import annotations

import unittest

from client.gui.state import GuiState
from client.library_models import DeviceInfo


class GuiStateTests(unittest.TestCase):
    def test_button_matrix_busy_connected_ssid(self) -> None:
        state = GuiState(
            target_name="Yundrone",
            scan_timeout=8,
            wait_timeout=12,
            verbose=False,
            ssid="LabWiFi",
            selected_device=DeviceInfo(name="d", address="11:22:33:44:55:66"),
            session_connected=True,
            busy=True,
            busy_task="scan",
            scan_mode="scanning",
        )
        self.assertFalse(state.can_scan())
        self.assertTrue(state.can_connect())
        self.assertFalse(state.can_disconnect())
        self.assertFalse(state.can_provision())
        self.assertFalse(state.can_run_diagnostic())

    def test_busy_non_scan_disables_connect(self) -> None:
        state = GuiState(
            target_name="Yundrone",
            scan_timeout=8,
            wait_timeout=12,
            verbose=False,
            selected_device=DeviceInfo(name="d", address="11:22:33:44:55:66"),
            busy=True,
            busy_task="connect",
            scan_mode="connecting",
        )
        self.assertFalse(state.can_connect())

    def test_connected_state_enables_actions(self) -> None:
        state = GuiState(
            target_name="Yundrone",
            scan_timeout=8,
            wait_timeout=12,
            verbose=False,
            ssid="LabWiFi",
            selected_device=DeviceInfo(name="d", address="11:22:33:44:55:66"),
            session_connected=True,
            busy=False,
        )
        self.assertTrue(state.can_scan())
        self.assertTrue(state.can_connect())
        self.assertTrue(state.can_disconnect())
        self.assertTrue(state.can_provision())
        self.assertTrue(state.can_run_diagnostic())

    def test_disconnected_state_disables_dependent_actions(self) -> None:
        state = GuiState(
            target_name="Yundrone",
            scan_timeout=8,
            wait_timeout=12,
            verbose=False,
            ssid="LabWiFi",
            selected_device=DeviceInfo(name="d", address="11:22:33:44:55:66"),
            session_connected=False,
            busy=False,
        )
        self.assertTrue(state.can_scan())
        self.assertTrue(state.can_connect())
        self.assertFalse(state.can_disconnect())
        self.assertFalse(state.can_provision())
        self.assertFalse(state.can_run_diagnostic())


if __name__ == "__main__":
    unittest.main()
