"""UART command protocol models and codec helpers."""

from __future__ import annotations

import json
import uuid
from dataclasses import dataclass
from typing import Any

PROTOCOL_VERSION = "YundroneBT-V1.0.0"

CODE_OK = "OK"
CODE_BAD_JSON = "BAD_JSON"
CODE_BAD_REQUEST = "BAD_REQUEST"
CODE_UNKNOWN_COMMAND = "UNKNOWN_COMMAND"
CODE_BUSY = "BUSY"
CODE_IN_PROGRESS = "IN_PROGRESS"
CODE_PROVISION_SUCCESS = "PROVISION_SUCCESS"
CODE_PROVISION_FAIL = "PROVISION_FAIL"
CODE_INTERNAL_ERROR = "INTERNAL_ERROR"
CODE_TIMEOUT = "TIMEOUT"


@dataclass(frozen=True)
class CommandRequest:
    request_id: str
    command: str
    args: dict[str, Any]


@dataclass(frozen=True)
class CommandResponse:
    request_id: str
    ok: bool
    code: str
    text: str
    data: dict[str, Any] | None = None


class CommandParseError(ValueError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


def new_request_id() -> str:
    return str(uuid.uuid4())


def command_request(command: str, args: dict[str, Any] | None = None, request_id: str | None = None) -> CommandRequest:
    if command.strip() == "":
        raise CommandParseError(CODE_BAD_REQUEST, "`cmd` must be non-empty")
    return CommandRequest(
        request_id=request_id or new_request_id(),
        command=command.strip(),
        args=args or {},
    )


def parse_request(raw: bytes | bytearray | memoryview | str) -> CommandRequest:
    text = _decode_raw_text(raw)
    try:
        payload = json.loads(text)
    except json.JSONDecodeError as exc:
        raise CommandParseError(CODE_BAD_JSON, f"invalid JSON: {exc.msg}") from exc
    if not isinstance(payload, dict):
        raise CommandParseError(CODE_BAD_REQUEST, "payload must be an object")

    cmd = payload.get("cmd")
    if not isinstance(cmd, str) or cmd.strip() == "":
        raise CommandParseError(CODE_BAD_REQUEST, "field `cmd` is required and must be non-empty string")

    req_id = payload.get("id")
    if req_id is None:
        req_id = new_request_id()
    if not isinstance(req_id, str) or req_id.strip() == "":
        raise CommandParseError(CODE_BAD_REQUEST, "field `id` must be string when present")

    args = payload.get("args", {})
    if not isinstance(args, dict):
        raise CommandParseError(CODE_BAD_REQUEST, "field `args` must be object")

    return CommandRequest(request_id=req_id.strip(), command=cmd.strip(), args=args)


def encode_request(request: CommandRequest) -> bytes:
    body: dict[str, Any] = {
        "id": request.request_id,
        "cmd": request.command,
        "args": request.args,
        "v": PROTOCOL_VERSION,
    }
    return json.dumps(body, ensure_ascii=False).encode("utf-8")


def response_ok(request_id: str, text: str, data: dict[str, Any] | None = None) -> CommandResponse:
    return CommandResponse(request_id=request_id, ok=True, code=CODE_OK, text=text, data=data)


def response_error(request_id: str, code: str, text: str) -> CommandResponse:
    return CommandResponse(request_id=request_id, ok=False, code=code, text=text)


def response_status(
    request_id: str,
    code: str,
    text: str,
    *,
    final: bool,
    data: dict[str, Any] | None = None,
) -> CommandResponse:
    body = dict(data or {})
    body["final"] = final
    return CommandResponse(request_id=request_id, ok=(code != CODE_PROVISION_FAIL), code=code, text=text, data=body)


def encode_response(response: CommandResponse) -> bytes:
    body: dict[str, Any] = {
        "id": response.request_id,
        "ok": response.ok,
        "code": response.code,
        "text": response.text,
        "v": PROTOCOL_VERSION,
    }
    if response.data is not None:
        body["data"] = response.data
    return json.dumps(body, ensure_ascii=False).encode("utf-8")


def parse_response(raw: bytes | bytearray | memoryview | str) -> CommandResponse:
    text = _decode_raw_text(raw)
    try:
        payload = json.loads(text)
    except json.JSONDecodeError as exc:
        raise CommandParseError(CODE_BAD_JSON, f"invalid JSON: {exc.msg}") from exc
    if not isinstance(payload, dict):
        raise CommandParseError(CODE_BAD_REQUEST, "response payload must be an object")

    req_id = payload.get("id")
    ok = payload.get("ok")
    code = payload.get("code")
    body = payload.get("text")
    data = payload.get("data")

    if not isinstance(req_id, str) or req_id.strip() == "":
        raise CommandParseError(CODE_BAD_REQUEST, "field `id` is required in response")
    if not isinstance(ok, bool):
        raise CommandParseError(CODE_BAD_REQUEST, "field `ok` is required in response")
    if not isinstance(code, str) or code.strip() == "":
        raise CommandParseError(CODE_BAD_REQUEST, "field `code` is required in response")
    if not isinstance(body, str):
        raise CommandParseError(CODE_BAD_REQUEST, "field `text` is required in response")
    if data is not None and not isinstance(data, dict):
        raise CommandParseError(CODE_BAD_REQUEST, "field `data` must be object when present")

    return CommandResponse(request_id=req_id.strip(), ok=ok, code=code.strip(), text=body, data=data)


def _decode_raw_text(raw: bytes | bytearray | memoryview | str) -> str:
    if isinstance(raw, str):
        return raw
    if isinstance(raw, memoryview):
        raw = raw.tobytes()
    return bytes(raw).decode("utf-8", errors="replace")
