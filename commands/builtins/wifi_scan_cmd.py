from __future__ import annotations

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandArgSpec, CommandSpec
from protocol.command_ids import CMD_WIFI_SCAN
from protocol.envelope import CODE_INTERNAL_ERROR, CommandRequest, CommandResponse, response_error, response_ok

SPEC = CommandSpec(
    name=CMD_WIFI_SCAN,
    summary="Scan nearby Wi-Fi SSIDs for 5 seconds",
    usage="wifi.scan {ifname?: str}",
    permission="operator",
    risk="medium",
    timeout_sec=15.0,
    args=(
        CommandArgSpec(name="ifname", type_name="str", required=False, description="Wi-Fi interface (default server ifname)"),
    ),
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(context: DispatchContext, request: CommandRequest) -> CommandResponse:
        ifname_raw = request.args.get("ifname")
        ifname = str(ifname_raw).strip() if isinstance(ifname_raw, str) else None
        ok, text = await context.run_system_command(CMD_WIFI_SCAN, ifname or None, SPEC.timeout_sec)
        if not ok:
            return response_error(request.request_id, CODE_INTERNAL_ERROR, text)
        return response_ok(request.request_id, text)

    dispatcher.register(SPEC, _handler)
