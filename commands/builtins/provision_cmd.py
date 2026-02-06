from __future__ import annotations

import string

from commands.registry import CommandDispatcher, DispatchContext
from commands.schemas import CommandArgSpec, CommandSpec
from protocol.command_ids import CMD_PROVISION
from protocol.envelope import CODE_BAD_REQUEST, CODE_BUSY, CODE_IN_PROGRESS, CommandRequest, CommandResponse, response_error, response_status

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
        if not _is_valid_wifi_password(password):
            return response_error(
                request.request_id,
                CODE_BAD_REQUEST,
                "Invalid Wi-Fi password: use empty(open), 8-63 chars, or 64 hex chars.",
            )
        accepted = await context.start_provision(request.request_id, ssid, password)
        if not accepted:
            return response_status(request.request_id, CODE_BUSY, "Provisioning in progress", final=True)
        return response_status(request.request_id, CODE_IN_PROGRESS, f"Provision accepted for SSID={ssid}", final=False)

    dispatcher.register(SPEC, _handler)


def _is_valid_wifi_password(password: str) -> bool:
    if password == "":
        return True
    if 8 <= len(password) <= 63:
        return True
    if len(password) == 64 and all(ch in string.hexdigits for ch in password):
        return True
    return False
