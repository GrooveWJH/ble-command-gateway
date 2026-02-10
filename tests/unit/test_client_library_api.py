from __future__ import annotations

import types
import unittest
from unittest.mock import AsyncMock, patch

from client.library_api import (
    BleGatewayClient,
    SessionHandle,
    SyncBleGatewayClient,
)
from client.library_models import DeviceInfo, GatewayError, GatewayErrorCode
from client.models import ResultCode, RunResult


class BleGatewayClientAsyncTests(unittest.IsolatedAsyncioTestCase):
    async def test_scan_returns_device_info(self) -> None:
        device = types.SimpleNamespace(
            name="Yundrone_UAV",
            address="AA:BB:CC:DD:EE:FF",
            adv_name="Yundrone_UAV",
            adv_uuids=["service-a"],
        )
        with patch(
            "client.library_api.discover_devices_with_progress",
            new=AsyncMock(return_value=([device], [device], 1)),
        ):
            gateway = BleGatewayClient(target_name="Yundrone")
            matched = await gateway.scan(timeout=6)

        self.assertEqual(len(matched), 1)
        self.assertEqual(matched[0].address, "AA:BB:CC:DD:EE:FF")
        self.assertEqual(matched[0].adv_uuids, ("service-a",))

    async def test_connect_failure_raises_gateway_error(self) -> None:
        with patch(
            "client.library_api.open_device_session",
            new=AsyncMock(return_value=(RunResult(ResultCode.FAILED, "connect failed"), None)),
        ):
            gateway = BleGatewayClient()
            with self.assertRaises(GatewayError) as ctx:
                await gateway.connect(DeviceInfo(name="d", address="11:22:33:44:55:66"))

        self.assertEqual(ctx.exception.code, GatewayErrorCode.CONNECT_FAILED)

    async def test_connect_accepts_device_address(self) -> None:
        fake_client = types.SimpleNamespace(is_connected=True)
        with patch(
            "client.library_api.open_device_session",
            new=AsyncMock(return_value=(RunResult(ResultCode.SUCCESS, "ok"), fake_client)),
        ) as patched_open:
            gateway = BleGatewayClient()
            session = await gateway.connect("11:22:33:44:55:66")
        self.assertIsNotNone(session)
        self.assertEqual(patched_open.await_args.kwargs["timeout"], gateway.connect_timeout)

    async def test_scan_snapshot_passes_stop_event(self) -> None:
        stop_event = types.SimpleNamespace(is_set=lambda: False)
        with patch(
            "client.library_api.discover_devices_with_progress",
            new=AsyncMock(return_value=([], [], 0)),
        ) as patched_discover:
            gateway = BleGatewayClient(target_name="Yundrone")
            await gateway.scan_snapshot(timeout=6, stop_event=stop_event)
        self.assertIs(patched_discover.await_args.kwargs["stop_event"], stop_event)


class SessionHandleTests(unittest.IsolatedAsyncioTestCase):
    async def test_run_command_success(self) -> None:
        fake_client = types.SimpleNamespace(is_connected=True)
        session = SessionHandle(
            _device=DeviceInfo(name="d", address="11:22:33:44:55:66"),
            _client=fake_client,
            _wait_timeout=8,
            _verbose=False,
            _reporter=None,
        )
        with patch(
            "client.library_api.run_command",
            new=AsyncMock(return_value=RunResult(ResultCode.SUCCESS, "pong", data={"status": {"wifi": "ok"}})),
        ):
            result = await session.run_command("ping")

        self.assertTrue(result.ok)
        self.assertEqual(result.message, "pong")
        self.assertEqual(result.data, {"status": {"wifi": "ok"}})

    async def test_provision_invalid_password(self) -> None:
        fake_client = types.SimpleNamespace(is_connected=True)
        session = SessionHandle(
            _device=DeviceInfo(name="d", address="11:22:33:44:55:66"),
            _client=fake_client,
            _wait_timeout=8,
            _verbose=False,
            _reporter=None,
        )
        with self.assertRaises(GatewayError) as ctx:
            await session.provision("LabWiFi", "short")

        self.assertEqual(ctx.exception.code, GatewayErrorCode.INVALID_ARGUMENT)

    async def test_close_clears_client_reference(self) -> None:
        fake_client = types.SimpleNamespace(is_connected=True)
        session = SessionHandle(
            _device=DeviceInfo(name="d", address="11:22:33:44:55:66"),
            _client=fake_client,
            _wait_timeout=8,
            _verbose=False,
            _reporter=None,
        )
        with patch("client.library_api.close_device_session", new=AsyncMock()) as patched_close:
            await session.close()
        patched_close.assert_awaited_once_with(fake_client)
        self.assertIsNone(session._client)

    async def test_run_command_clears_client_when_disconnected(self) -> None:
        fake_client = types.SimpleNamespace(is_connected=False)
        session = SessionHandle(
            _device=DeviceInfo(name="d", address="11:22:33:44:55:66"),
            _client=fake_client,
            _wait_timeout=8,
            _verbose=False,
            _reporter=None,
        )
        with patch(
            "client.library_api.run_command",
            new=AsyncMock(return_value=RunResult(ResultCode.SUCCESS, "pong")),
        ):
            await session.run_command("ping")
        self.assertIsNone(session._client)


class SyncFacadeTests(unittest.TestCase):
    def test_sync_scan(self) -> None:
        device = types.SimpleNamespace(
            name="Yundrone_UAV",
            address="AA:BB:CC:DD:EE:FF",
            adv_name="Yundrone_UAV",
            adv_uuids=[],
        )
        with patch(
            "client.library_api.discover_devices_with_progress",
            new=AsyncMock(return_value=([device], [device], 1)),
        ):
            gateway = SyncBleGatewayClient(target_name="Yundrone")
            try:
                matched = gateway.scan(timeout=3)
            finally:
                gateway.close()

        self.assertEqual(len(matched), 1)
        self.assertEqual(matched[0].name, "Yundrone_UAV")

    def test_sync_client_closed_state(self) -> None:
        gateway = SyncBleGatewayClient(target_name="Yundrone")
        gateway.close()
        with self.assertRaises(RuntimeError):
            gateway.scan(timeout=1)

    def test_sync_session_close_is_idempotent(self) -> None:
        fake_client = types.SimpleNamespace(is_connected=True)
        with (
            patch(
                "client.library_api.open_device_session",
                new=AsyncMock(return_value=(RunResult(ResultCode.SUCCESS, "ok"), fake_client)),
            ),
            patch("client.library_api.close_device_session", new=AsyncMock()) as patched_close,
        ):
            gateway = SyncBleGatewayClient(target_name="Yundrone")
            try:
                session = gateway.connect("11:22:33:44:55:66")
                session.close()
                session.close()
            finally:
                gateway.close()
        self.assertGreaterEqual(patched_close.await_count, 1)


if __name__ == "__main__":
    unittest.main()
