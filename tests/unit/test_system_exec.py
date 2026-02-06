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
