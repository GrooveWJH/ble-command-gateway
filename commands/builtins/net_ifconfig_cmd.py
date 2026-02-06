from __future__ import annotations

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandArgSpec, CommandSpec
from protocol.command_ids import CMD_NET_IFCONFIG
from protocol.envelope import CODE_INTERNAL_ERROR, CommandRequest, CommandResponse, response_error, response_ok

SPEC = CommandSpec(
    name=CMD_NET_IFCONFIG,
    summary="Show network interface config",
    usage="net.ifconfig [ifname]",
    permission="operator",
    risk="medium",
    timeout_sec=4.0,
    args=(
        CommandArgSpec(name="ifname", type_name="str", required=False, description="Interface name, e.g. wlan0"),
    ),
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(context: DispatchContext, request: CommandRequest) -> CommandResponse:
        ifname = request.args.get("ifname")
        if_name = None if not isinstance(ifname, str) else ifname.strip() or None
        ok, text = await context.run_system_command(CMD_NET_IFCONFIG, if_name, SPEC.timeout_sec)
        if not ok:
            return response_error(request.request_id, CODE_INTERNAL_ERROR, text)
        return response_ok(request.request_id, text)

    dispatcher.register(SPEC, _handler)
