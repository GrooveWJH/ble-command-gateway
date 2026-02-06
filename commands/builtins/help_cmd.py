from __future__ import annotations

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandArgSpec, CommandSpec
from protocol.command_ids import CMD_HELP
from protocol.envelope import CommandRequest, CommandResponse, response_ok

SPEC = CommandSpec(
    name=CMD_HELP,
    summary="Show command help text",
    usage="help [cmd]",
    permission="user",
    risk="low",
    timeout_sec=2.0,
    args=(
        CommandArgSpec(
            name="cmd",
            type_name="str",
            required=False,
            description="Specific command name for detailed help",
        ),
    ),
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(_context: DispatchContext, request: CommandRequest) -> CommandResponse:
        target = request.args.get("cmd")
        target_cmd = None if not isinstance(target, str) else target.strip() or None
        return response_ok(request.request_id, dispatcher.render_help(target_cmd))

    dispatcher.register(SPEC, _handler)
