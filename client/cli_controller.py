"""CLI-side session orchestration over high-level gateway APIs."""

from __future__ import annotations

from client.gateway_result_adapter import GatewayResultAdapter
from client.library_api import BleGatewayClient
from client.library_models import DeviceInfo, GatewayError
from client.models import ResultCode, RunResult, SessionState


class CLIController:
    def __init__(self, state: SessionState, gateway: BleGatewayClient) -> None:
        self._state = state
        self._gateway = gateway

    @property
    def state(self) -> SessionState:
        return self._state

    @property
    def has_active_session(self) -> bool:
        session = self._state.active_session
        if session is None:
            return False
        return bool(session.is_connected)

    async def close_active_session(self) -> None:
        if self._state.active_session is None:
            return
        await self._state.active_session.close()
        self._state.active_session = None

    async def connect_device(self, device: DeviceInfo) -> RunResult:
        await self.close_active_session()
        self._state.selected_device = device
        try:
            self._state.active_session = await self._gateway.connect(device)
            return RunResult(ResultCode.SUCCESS, "已连接并验证服务")
        except GatewayError as exc:
            self._state.selected_device = None
            self._state.active_session = None
            return GatewayResultAdapter.from_gateway_error(exc)

    async def provision_current(
        self,
        ssid: str,
        password: str,
        timeout: int,
        verbose: bool,
    ) -> RunResult:
        session = self._state.active_session
        if session is None:
            return RunResult(ResultCode.NOT_FOUND, "请先扫描并选择设备")
        if not ssid:
            return RunResult(ResultCode.INPUT_ERROR, "请先设置 Wi-Fi SSID")
        try:
            result = await session.provision(
                ssid=ssid,
                password=password,
                timeout=timeout,
                verbose=verbose,
            )
            if not session.is_connected:
                self._state.active_session = None
            return result.to_run_result()
        except GatewayError as exc:
            if self._state.active_session is not None and not self._state.active_session.is_connected:
                self._state.active_session = None
            return GatewayResultAdapter.from_gateway_error(exc)

    async def run_current_command(self, command: str, timeout: int) -> RunResult:
        session = self._state.active_session
        if session is None:
            return RunResult(ResultCode.NOT_FOUND, "请先扫描并选择设备")
        try:
            result = await session.run_command(command=command, args={}, timeout=timeout)
            if not session.is_connected:
                self._state.active_session = None
            return result.to_run_result()
        except GatewayError as exc:
            if self._state.active_session is not None and not self._state.active_session.is_connected:
                self._state.active_session = None
            return GatewayResultAdapter.from_gateway_error(exc)
