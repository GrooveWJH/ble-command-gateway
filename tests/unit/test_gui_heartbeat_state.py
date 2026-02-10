from __future__ import annotations

import unittest

from client.gui.state import GuiState


class GuiHeartbeatStateTests(unittest.TestCase):
    def _make_state(self) -> GuiState:
        return GuiState(
            target_name="Yundrone",
            scan_timeout=8,
            wait_timeout=12,
            verbose=False,
            session_connected=True,
        )

    def test_heartbeat_success_resets_failures(self) -> None:
        state = self._make_state()
        state.heartbeat_failures = 1
        state.mark_heartbeat_success(100.0, interval_sec=5.0)

        self.assertEqual(state.heartbeat_failures, 0)
        self.assertFalse(state.heartbeat_inflight)
        self.assertEqual(state.heartbeat_next_due_at, 105.0)

    def test_single_timeout_does_not_disconnect(self) -> None:
        state = self._make_state()
        disconnected = state.mark_heartbeat_failure(100.0, interval_sec=5.0, fail_limit=2)

        self.assertFalse(disconnected)
        self.assertEqual(state.heartbeat_failures, 1)
        self.assertEqual(state.heartbeat_next_due_at, 105.0)

    def test_second_timeout_triggers_disconnect_threshold(self) -> None:
        state = self._make_state()
        state.mark_heartbeat_failure(100.0, interval_sec=5.0, fail_limit=2)
        disconnected = state.mark_heartbeat_failure(106.0, interval_sec=5.0, fail_limit=2)

        self.assertTrue(disconnected)
        self.assertEqual(state.heartbeat_failures, 2)
        self.assertEqual(state.heartbeat_next_due_at, 111.0)

    def test_heartbeat_due_requires_connected_and_idle(self) -> None:
        state = self._make_state()
        state.arm_heartbeat(50.0, interval_sec=5.0)
        self.assertFalse(state.heartbeat_due(54.9))
        self.assertTrue(state.heartbeat_due(55.0))

        state.mark_heartbeat_submitted()
        self.assertFalse(state.heartbeat_due(56.0))
        state.session_connected = False
        state.heartbeat_inflight = False
        self.assertFalse(state.heartbeat_due(60.0))


if __name__ == "__main__":
    unittest.main()
