from __future__ import annotations

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandSpec
from protocol.command_ids import CMD_SHUTDOWN
from protocol.envelope import CommandRequest, CommandResponse, response_ok

SPEC = CommandSpec(
    name=CMD_SHUTDOWN,
    summary="Gracefully stop provisioning server",
    usage="shutdown",
    permission="operator",
    risk="high",
    timeout_sec=2.0,
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(context: DispatchContext, request: CommandRequest) -> CommandResponse:
        await context.start_shutdown(request.request_id)
        return response_ok(request.request_id, "Shutdown scheduled")

    dispatcher.register(SPEC, _handler)
