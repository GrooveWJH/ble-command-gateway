from __future__ import annotations

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandArgSpec, CommandSpec
from protocol.command_ids import CMD_PROVISION
from protocol.envelope import CommandRequest, CommandResponse, response_ok

SPEC = CommandSpec(
    name=CMD_PROVISION,
    summary="Configure Wi-Fi credentials and start provisioning",
    usage="provision {ssid: str, pwd?: str}",
    permission="user",
    risk="medium",
    timeout_sec=3.0,
    args=(
        CommandArgSpec(name="ssid", type_name="str", required=True, description="Wi-Fi SSID"),
        CommandArgSpec(name="pwd", type_name="str", required=False, description="Wi-Fi password"),
    ),
)


def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(context: DispatchContext, request: CommandRequest) -> CommandResponse:
        ssid = str(request.args.get("ssid", "")).strip()
        password = str(request.args.get("pwd", ""))
        await context.start_provision(request.request_id, ssid, password)
        return response_ok(request.request_id, f"Provision task accepted for SSID={ssid}")

    dispatcher.register(SPEC, _handler)
