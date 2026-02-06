from __future__ import annotations

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandSpec
from protocol.command_ids import CMD_PING
from protocol.envelope import CommandRequest, CommandResponse, response_ok

SPEC = CommandSpec(
    name=CMD_PING,
    summary="BLE link health check",
    usage="ping",
    permission="user",
    risk="low",
    timeout_sec=2.0,
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(_context: DispatchContext, request: CommandRequest) -> CommandResponse:
        return response_ok(request.request_id, "pong")

    dispatcher.register(SPEC, _handler)
