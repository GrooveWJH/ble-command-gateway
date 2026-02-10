from __future__ import annotations

import types
import unittest
from unittest.mock import AsyncMock, patch

from client.command_client import provision_device, run_command
from client.models import ResultCode, RunResult
from protocol.envelope import CommandResponse


class CommandClientReportingTests(unittest.IsolatedAsyncioTestCase):
    async def test_run_command_without_reporter_has_no_print_side_effect(self) -> None:
        fake_client = types.SimpleNamespace(is_connected=True)
        with (
            patch("client.command_client.verify_target_service", new=AsyncMock(return_value=None)),
            patch("client.command_client.write_with_fallback", new=AsyncMock(return_value="write-with-response")),
            patch(
                "client.command_client.wait_response",
                new=AsyncMock(return_value=CommandResponse("req-1", True, "OK", "pong", data={"status": {"ip": "1.2.3.4"}})),
            ),
            patch("builtins.print") as patched_print,
        ):
            result = await run_command(
                device="11:22:33:44:55:66",
                command="ping",
                args={},
                wait_timeout=5,
                reporter=None,
                client=fake_client,
            )

        self.assertEqual(result.code, ResultCode.SUCCESS)
        self.assertEqual(result.message, "pong")
        self.assertEqual(result.data, {"status": {"ip": "1.2.3.4"}})
        patched_print.assert_not_called()

    async def test_provision_reports_events_via_reporter(self) -> None:
        fake_client = types.SimpleNamespace(is_connected=True)
        fake_device = types.SimpleNamespace(name="Yundrone", address="11:22:33:44:55:66")
        events: list[str] = []

        with (
            patch("client.command_client.verify_target_service", new=AsyncMock(return_value=None)),
            patch("client.command_client.write_with_fallback", new=AsyncMock(return_value="write-with-response")),
            patch(
                "client.command_client.wait_status",
                new=AsyncMock(return_value=RunResult(ResultCode.SUCCESS, "配网成功")),
            ),
        ):
            result = await provision_device(
                device=fake_device,
                ssid="LabWiFi",
                password="12345678",
                wait_timeout=10,
                verbose=False,
                reporter=events.append,
                client=fake_client,
            )

        self.assertEqual(result.code, ResultCode.SUCCESS)
        self.assertGreaterEqual(len(events), 3)
        self.assertIn("[开始]", events[0])
        self.assertTrue(any("[发送]" in event for event in events))


if __name__ == "__main__":
    unittest.main()
