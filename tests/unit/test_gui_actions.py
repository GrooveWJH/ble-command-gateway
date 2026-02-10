from __future__ import annotations

import unittest
from unittest.mock import Mock

from client.gui.actions import connect_selected, provision_current, run_diagnostic, run_heartbeat, scan_devices
from client.library_models import CommandResult, DeviceInfo, GatewayError, GatewayErrorCode, ScanSnapshot
from client.models import ResultCode
from protocol.command_ids import CMD_STATUS, CMD_WIFI_SCAN


class GuiActionsTests(unittest.TestCase):
    def test_scan_devices_no_match_returns_named_devices(self) -> None:
        gateway = Mock()
        device = DeviceInfo(name="Yundrone_01", address="11:22:33:44:55:66")
        snapshot = ScanSnapshot(
            devices=(device,),
            matched=(),
            total_count=1,
        )
        gateway.scan_snapshot.return_value = snapshot

        result = scan_devices(gateway, target_name="Target", timeout=8)

        self.assertTrue(result.ok)
        self.assertEqual(result.code, ResultCode.SUCCESS)
        self.assertEqual(result.payload["display_devices"], [device])
        self.assertEqual(result.payload["auto_selected"], device)

    def test_scan_devices_single_match_auto_select(self) -> None:
        gateway = Mock()
        device = DeviceInfo(name="Yundrone_01", address="11:22:33:44:55:66")
        snapshot = ScanSnapshot(
            devices=(device,),
            matched=(device,),
            total_count=1,
        )
        gateway.scan_snapshot.return_value = snapshot

        result = scan_devices(gateway, target_name="Yundrone", timeout=8)

        self.assertTrue(result.ok)
        self.assertEqual(result.payload["auto_selected"], device)
        self.assertEqual(result.payload["auto_connect_device"], device)
        self.assertIn("on_progress", gateway.scan_snapshot.call_args.kwargs)
        self.assertIn("on_detect", gateway.scan_snapshot.call_args.kwargs)

    def test_scan_devices_passes_stop_event(self) -> None:
        gateway = Mock()
        stop_event = Mock()
        gateway.scan_snapshot.return_value = ScanSnapshot(devices=(), matched=(), total_count=0)

        _ = scan_devices(gateway, target_name="Yundrone", timeout=8, stop_event=stop_event)

        self.assertIs(gateway.scan_snapshot.call_args.kwargs["stop_event"], stop_event)

    def test_connect_selected_handles_gateway_error(self) -> None:
        gateway = Mock()
        gateway.connect.side_effect = GatewayError(
            code=GatewayErrorCode.CONNECT_FAILED,
            message="connect failed",
            retryable=True,
        )
        device = DeviceInfo(name="Yundrone_01", address="11:22:33:44:55:66")

        result = connect_selected(gateway, device)

        self.assertFalse(result.ok)
        self.assertEqual(result.code, ResultCode.FAILED)
        self.assertEqual(result.message, "connect failed")

    def test_provision_validation(self) -> None:
        session = Mock()
        result = provision_current(session, ssid="", password="12345678", timeout=8, verbose=False)
        self.assertFalse(result.ok)
        self.assertEqual(result.code, ResultCode.INPUT_ERROR)

        result = provision_current(session, ssid="LabWiFi", password="short", timeout=8, verbose=False)
        self.assertFalse(result.ok)
        self.assertEqual(result.code, ResultCode.INPUT_ERROR)

    def test_run_diagnostic_wifi_scan_parses_rows(self) -> None:
        session = Mock()
        session.run_command.return_value = CommandResult(
            code=ResultCode.SUCCESS,
            message=(
                '{"ifname":"wlan0","count":3,"aps":['
                '{"ssid":"LabWiFi","chan":"11","signal":73},'
                '{"ssid":"LabWiFi","chan":"6","signal":88},'
                '{"ssid":"Guest","chan":"1","signal":40}'
                "]} "
            ),
            command=CMD_WIFI_SCAN,
        )

        result = run_diagnostic(session, command_id=CMD_WIFI_SCAN, timeout=8)

        self.assertTrue(result.ok)
        self.assertIn("raw_text", result.payload)
        self.assertIn("wifi_scan_rows", result.payload)
        self.assertIn("wifi_scan_aggregated_rows", result.payload)
        rows = result.payload["wifi_scan_rows"]
        self.assertIsInstance(rows, list)
        self.assertEqual(rows[0]["ssid"], "LabWiFi")
        self.assertEqual(rows[0]["signal"], 73)
        agg_rows = result.payload["wifi_scan_aggregated_rows"]
        self.assertEqual(len(agg_rows), 2)
        self.assertEqual(agg_rows[0]["ssid"], "LabWiFi")
        self.assertEqual(agg_rows[0]["signal"], 88)

    def test_run_diagnostic_status_parses_rows(self) -> None:
        session = Mock()
        session.run_command.return_value = CommandResult(
            code=ResultCode.SUCCESS,
            message=(
                "Wi-Fi: connected (SSID=LabWiFi)\n"
                "IP: 192.168.1.2\n"
                "User: root\n"
                "Hostname: orin\n"
                "SSH: ssh (enabled=yes, active=active)\n"
                "System: Linux 6.8"
            ),
            command=CMD_STATUS,
        )

        result = run_diagnostic(session, command_id=CMD_STATUS, timeout=8)

        self.assertTrue(result.ok)
        self.assertIn("raw_text", result.payload)
        self.assertIn("status_rows", result.payload)
        status_rows = result.payload["status_rows"]
        self.assertIsInstance(status_rows, list)
        self.assertEqual(status_rows[0]["key"], "Wi-Fi")
        self.assertIn("connected", status_rows[0]["value"])
        self.assertEqual(status_rows[1]["key"], "IP")
        self.assertEqual(status_rows[1]["value"], "192.168.1.2")

    def test_run_diagnostic_status_old_wifi_ip_format_is_backward_compatible(self) -> None:
        session = Mock()
        session.run_command.return_value = CommandResult(
            code=ResultCode.SUCCESS,
            message=(
                "Wi-Fi: connected (SSID=LabWiFi, IP=192.168.1.2)\n"
                "User: root\n"
                "Hostname: orin\n"
                "SSH: ssh (enabled=yes, active=active)\n"
                "System: Linux 6.8"
            ),
            command=CMD_STATUS,
        )

        result = run_diagnostic(session, command_id=CMD_STATUS, timeout=8)

        self.assertTrue(result.ok)
        status_rows = result.payload["status_rows"]
        self.assertEqual(status_rows[0], {"key": "Wi-Fi", "value": "connected (SSID=LabWiFi)"})
        self.assertEqual(status_rows[1], {"key": "IP", "value": "192.168.1.2"})

    def test_run_diagnostic_status_prefers_structured_data_payload(self) -> None:
        session = Mock()
        session.run_command.return_value = CommandResult(
            code=ResultCode.SUCCESS,
            message="legacy-text-without-colon",
            command=CMD_STATUS,
            data={
                "status": {
                    "wifi": "connected (SSID=LabWiFi)",
                    "ip": "192.168.1.2",
                    "user": "root",
                    "hostname": "orin",
                    "ssh": "ssh (enabled=yes, active=active)",
                    "system": "Linux 6.8",
                }
            },
        )

        result = run_diagnostic(session, command_id=CMD_STATUS, timeout=8)

        self.assertTrue(result.ok)
        status_rows = result.payload["status_rows"]
        self.assertEqual(
            status_rows,
            [
                {"key": "Wi-Fi", "value": "connected (SSID=LabWiFi)"},
                {"key": "IP", "value": "192.168.1.2"},
                {"key": "User", "value": "root"},
                {"key": "Hostname", "value": "orin"},
                {"key": "SSH", "value": "ssh (enabled=yes, active=active)"},
                {"key": "System", "value": "Linux 6.8"},
            ],
        )

    def test_run_diagnostic_status_parse_failure_keeps_raw(self) -> None:
        session = Mock()
        session.run_command.return_value = CommandResult(
            code=ResultCode.SUCCESS,
            message="no-colon-lines-only",
            command=CMD_STATUS,
        )

        result = run_diagnostic(session, command_id=CMD_STATUS, timeout=8)

        self.assertTrue(result.ok)
        self.assertIn("raw_text", result.payload)
        self.assertIsNone(result.payload.get("status_rows"))

    def test_run_heartbeat_success(self) -> None:
        session = Mock()
        session.run_command.return_value = CommandResult(
            code=ResultCode.SUCCESS,
            message="pong",
            command="ping",
        )

        result = run_heartbeat(session, timeout=5)

        self.assertTrue(result.ok)
        self.assertEqual(result.code, ResultCode.SUCCESS)
        self.assertEqual(result.message, "pong")
        self.assertEqual(result.payload.get("command"), "ping")

    def test_run_heartbeat_timeout(self) -> None:
        session = Mock()
        session.run_command.return_value = CommandResult(
            code=ResultCode.TIMEOUT,
            message="timeout",
            command="ping",
        )

        result = run_heartbeat(session, timeout=5)

        self.assertFalse(result.ok)
        self.assertEqual(result.code, ResultCode.TIMEOUT)
        self.assertEqual(result.message, "timeout")


if __name__ == "__main__":
    unittest.main()
