"""Explicit adapters between gateway-level errors and CLI run results."""

from __future__ import annotations

from client.library_models import GatewayError, GatewayErrorCode
from client.models import ResultCode, RunResult

_ERROR_TO_RESULT: dict[GatewayErrorCode, ResultCode] = {
    GatewayErrorCode.TIMEOUT: ResultCode.TIMEOUT,
    GatewayErrorCode.NOT_FOUND: ResultCode.NOT_FOUND,
    GatewayErrorCode.INVALID_ARGUMENT: ResultCode.INPUT_ERROR,
    GatewayErrorCode.CONNECT_FAILED: ResultCode.FAILED,
    GatewayErrorCode.DISCONNECTED: ResultCode.FAILED,
    GatewayErrorCode.COMMAND_FAILED: ResultCode.FAILED,
    GatewayErrorCode.BLE_ERROR: ResultCode.FAILED,
}


class GatewayResultAdapter:
    @staticmethod
    def from_gateway_error(error: GatewayError) -> RunResult:
        code = _ERROR_TO_RESULT.get(error.code, ResultCode.FAILED)
        return RunResult(code=code, message=error.message)
