from __future__ import annotations

import unittest

from protocol.command_ids import (
    CMD_HELP,
    CMD_NET_IFCONFIG,
    CMD_PING,
    CMD_PROVISION,
    CMD_SHUTDOWN,
)
from commands.schemas import CommandSpec
from protocol.envelope import (
    CODE_BAD_REQUEST,
    CODE_UNKNOWN_COMMAND,
    command_request,
    parse_response,
    response_ok,
    CommandRequest,
    CommandResponse,
)
from ble.server_gateway import BLEProvisioningServer
from commands.registry import CommandDispatcher, DispatchContext


class CommandDispatcherTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self) -> None:
        self.provision_calls: list[tuple[str, str, str]] = []
        self.shutdown_calls: list[str] = []
        self.system_calls: list[tuple[str, str | None, float]] = []
        self.dispatcher = CommandDispatcher(
            DispatchContext(
                read_status_text=lambda: "Standby",
                start_provision=self._start_provision,
                start_shutdown=self._start_shutdown,
                run_system_command=self._run_system_command,
            ),
            logger=lambda _message: None,
        )
        from commands.loader import load_builtin_commands

        load_builtin_commands(self.dispatcher)

    async def _start_provision(self, request_id: str, ssid: str, pwd: str) -> bool:
        self.provision_calls.append((request_id, ssid, pwd))
        return True

    async def _start_shutdown(self, request_id: str) -> None:
        self.shutdown_calls.append(request_id)

    async def _run_system_command(
        self, command_name: str, ifname: str | None, timeout_sec: float
    ) -> tuple[bool, str]:
        self.system_calls.append((command_name, ifname, timeout_sec))
        return True, "ok"

    async def test_help_human_readable(self) -> None:
        resp = await self.dispatcher.dispatch(command_request(CMD_HELP, {}, "req-help"))
        self.assertTrue(resp.ok)
        self.assertIn("Available commands:", resp.text)
        self.assertIn("- provision", resp.text)
        self.assertIn('{"cmd":"provision"}', resp.text)

    async def test_help_command_details(self) -> None:
        resp = await self.dispatcher.dispatch(
            command_request(CMD_HELP, {"cmd": CMD_PROVISION}, "req-help-detail")
        )
        self.assertTrue(resp.ok)
        self.assertIn(f"Command: {CMD_PROVISION}", resp.text)
        self.assertIn("Usage:", resp.text)
        self.assertIn("Permission:", resp.text)
        self.assertIn("Risk:", resp.text)
        self.assertIn("Timeout:", resp.text)

    async def test_unknown_command(self) -> None:
        resp = await self.dispatcher.dispatch(
            command_request("nope", {}, "req-unknown")
        )
        self.assertFalse(resp.ok)
        self.assertEqual(resp.code, CODE_UNKNOWN_COMMAND)

    async def test_missing_ssid(self) -> None:
        resp = await self.dispatcher.dispatch(
            command_request(CMD_PROVISION, {"pwd": "x"}, "req-provision")
        )
        self.assertFalse(resp.ok)
        self.assertEqual(resp.code, CODE_BAD_REQUEST)

    async def test_invalid_short_password(self) -> None:
        resp = await self.dispatcher.dispatch(
            command_request(
                CMD_PROVISION,
                {"ssid": "LabWiFi", "pwd": "1234567"},
                "req-provision-pwd",
            )
        )
        self.assertFalse(resp.ok)
        self.assertEqual(resp.code, CODE_BAD_REQUEST)
        self.assertIn("Invalid Wi-Fi password", resp.text)

    async def test_shutdown_allowed(self) -> None:
        resp = await self.dispatcher.dispatch(
            command_request(CMD_SHUTDOWN, {}, "req-shutdown")
        )
        self.assertTrue(resp.ok)
        self.assertEqual(self.shutdown_calls, ["req-shutdown"])

    async def test_net_ifconfig_is_unified_name(self) -> None:
        resp = await self.dispatcher.dispatch(
            command_request(CMD_NET_IFCONFIG, {}, "req-ifconfig")
        )
        self.assertTrue(resp.ok)
        self.assertEqual(self.system_calls[0][0], CMD_NET_IFCONFIG)

    async def test_duplicate_registration_is_rejected(self) -> None:
        async def _noop_handler(
            _ctx: DispatchContext, req: CommandRequest
        ) -> CommandResponse:
            return response_ok(req.request_id, "ok")

        with self.assertRaises(ValueError):
            self.dispatcher.register(
                CommandSpec(name=CMD_PING, summary="x", usage="x"), _noop_handler
            )


class ProvisionBusyBehaviorTests(unittest.IsolatedAsyncioTestCase):
    async def test_busy_lock_returns_busy_response(self) -> None:
        server = BLEProvisioningServer("test", None, connect_timeout=1, adapter=None)
        await server._connect_lock.acquire()
        try:
            await server._provision_wifi("req-busy", "ssid", "pwd")
        finally:
            server._connect_lock.release()

        response = parse_response(server._publisher.last_payload)
        self.assertEqual(response.request_id, "req-busy")
        self.assertEqual(response.code, "BUSY")
        self.assertEqual((response.data or {}).get("final"), True)


if __name__ == "__main__":
    unittest.main()
