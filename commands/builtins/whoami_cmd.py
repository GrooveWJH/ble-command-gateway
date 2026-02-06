from __future__ import annotations

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandSpec
from protocol.command_ids import CMD_SYS_WHOAMI
from protocol.envelope import CODE_INTERNAL_ERROR, CommandRequest, CommandResponse, response_error, response_ok

SPEC = CommandSpec(
    name=CMD_SYS_WHOAMI,
    summary="Show current OS user",
    usage="sys.whoami",
    permission="operator",
    risk="medium",
    timeout_sec=3.0,
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(context: DispatchContext, request: CommandRequest) -> CommandResponse:
        ok, text = await context.run_system_command(CMD_SYS_WHOAMI, None, SPEC.timeout_sec)
        if not ok:
            return response_error(request.request_id, CODE_INTERNAL_ERROR, text)
        return response_ok(request.request_id, text)

    dispatcher.register(SPEC, _handler)
