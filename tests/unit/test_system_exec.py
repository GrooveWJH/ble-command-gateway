from __future__ import annotations

import unittest
from unittest.mock import patch

from protocol.command_ids import CMD_NET_IFCONFIG, CMD_SYS_WHOAMI
from services import system_exec_service as system_exec


class SystemExecResolverTests(unittest.TestCase):
    def test_resolve_ifconfig_bin_uses_valid_executable(self) -> None:
        with (
            patch("services.system_exec_service.shutil.which", return_value="/usr/bin/ifconfig"),
            patch("services.system_exec_service.os.path.isfile", return_value=True),
            patch("services.system_exec_service.os.access", return_value=True),
        ):
            self.assertEqual(system_exec._resolve_ifconfig_bin(), "/usr/bin/ifconfig")

    def test_resolve_ifconfig_bin_rejects_non_executable(self) -> None:
        with (
            patch("services.system_exec_service.shutil.which", return_value="/usr/bin/ifconfig"),
            patch("services.system_exec_service.os.path.isfile", return_value=True),
            patch("services.system_exec_service.os.access", return_value=False),
        ):
            self.assertIsNone(system_exec._resolve_ifconfig_bin())

    def test_ipv4_for_interface_falls_back_to_nmcli(self) -> None:
        with (
            patch("services.system_exec_service._resolve_ip_bin", return_value=None),
            patch(
                "services.system_exec_service.subprocess.check_output",
                return_value="IP4.ADDRESS[1]:192.168.10.198/24\n",
            ),
        ):
            self.assertEqual(system_exec._ipv4_for_interface("wlan0"), "192.168.10.198")

    def test_parse_wifi_scan_entries_sorts_and_filters_empty_ssid(self) -> None:
        raw = "\n".join(
            [
                r"*:44\:F7\:70\:3F\:3F\:83:Yundrone_MOffice:1:130 Mbit/s:90:▂▄▆█:WPA2",
                r":44\:F7\:70\:3F\:3F\:82:Yundrone_MOffice:40:270 Mbit/s:77:▂▄▆_:WPA2",
                r":CA\:6F\:B0\:06\:CB\:CD:Yundrone_Office:161:540 Mbit/s:34:▂▄__:WPA2",
                r":FA\:6F\:B0\:06\:CB\:C0:yundrone_Office_2.4G:1:540 Mbit/s:22:▂___:WPA2",
                r":4E\:F7\:70\:3F\:3F\:82::40:270 Mbit/s:77:▂▄▆_:WPA2",
            ]
        )
        parsed = system_exec._parse_wifi_scan_entries(raw)
        self.assertEqual(len(parsed), 4)
        self.assertEqual(parsed[0]["ssid"], "Yundrone_MOffice")
        self.assertEqual(parsed[0]["signal"], 90)
        self.assertEqual(parsed[0]["chan"], "1")
        self.assertEqual(parsed[1]["signal"], 77)
        self.assertEqual(
            [item["ssid"] for item in parsed],
            ["Yundrone_MOffice", "Yundrone_MOffice", "Yundrone_Office", "yundrone_Office_2.4G"],
        )


class SystemExecRoutingTests(unittest.IsolatedAsyncioTestCase):
    async def test_run_named_command_rejects_unknown(self) -> None:
        result = await system_exec.run_named_command("unknown.cmd", None, timeout_sec=1.0)
        self.assertFalse(result.ok)
        self.assertIn("unsupported system command", result.text)

    async def test_run_named_command_uses_constants(self) -> None:
        with patch("services.system_exec_service._run", return_value=system_exec.SystemExecResult(True, "ok")) as run_mock:
            await system_exec.run_named_command(CMD_SYS_WHOAMI, None, timeout_sec=1.0)
            run_mock.assert_called_once_with(["whoami"], 1.0)

        with (
            patch("services.system_exec_service._resolve_ifconfig_bin", return_value="/sbin/ifconfig"),
            patch("services.system_exec_service._run", return_value=system_exec.SystemExecResult(True, "ok")) as run_mock,
        ):
            await system_exec.run_named_command(CMD_NET_IFCONFIG, "wlan0", timeout_sec=2.0)
            run_mock.assert_called_once_with(["/sbin/ifconfig", "wlan0"], 2.0)

    async def test_run_named_command_prefers_sudo_user(self) -> None:
        with (
            patch.dict("services.system_exec_service.os.environ", {"SUDO_USER": "orangepi"}, clear=False),
            patch("services.system_exec_service._run") as run_mock,
        ):
            result = await system_exec.run_named_command(CMD_SYS_WHOAMI, None, timeout_sec=1.0)
        self.assertTrue(result.ok)
        self.assertEqual(result.text, "orangepi")
        run_mock.assert_not_called()
