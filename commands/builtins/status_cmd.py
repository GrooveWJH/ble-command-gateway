from __future__ import annotations

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandSpec
from protocol.command_ids import CMD_STATUS
from protocol.envelope import CommandRequest, CommandResponse, response_ok

SPEC = CommandSpec(
    name=CMD_STATUS,
    summary="Read current server status",
    usage="status",
    permission="user",
    risk="low",
    timeout_sec=2.0,
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(context: DispatchContext, request: CommandRequest) -> CommandResponse:
        return response_ok(request.request_id, context.read_status_text())

    dispatcher.register(SPEC, _handler)
