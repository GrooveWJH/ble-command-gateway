"""Shared helpers for hello-world BLE link test messages."""

from __future__ import annotations

import json
from typing import Final

from protocol.command_ids import CMD_PING

CLIENT_HELLO_PREFIX: Final = "hello from client_link_test.py"
SERVER_HELLO_PREFIX: Final = "hello from server_link_test.py"


def build_seq_token(seq: int, total: int) -> str:
    if total <= 0:
        raise ValueError("total must be > 0")
    if seq <= 0:
        raise ValueError("seq must be > 0")
    width = max(2, len(str(total)))
    return f"seq={seq:0{width}d}/{total:0{width}d}"


def build_client_payload(seq: int, total: int, prefix: str = CLIENT_HELLO_PREFIX) -> str:
    token = build_seq_token(seq, total)
    body = {
        "cmd": CMD_PING,
        "token": token,
        "text": f"{prefix} | {token}",
    }
    return json.dumps(body, ensure_ascii=False)


def build_server_reply(received_text: str, prefix: str = SERVER_HELLO_PREFIX) -> str:
    token = extract_seq_token(received_text)
    body = {
        "cmd": "pong",
        "token": token,
        "text": f"{prefix} | got: {received_text}",
    }
    return json.dumps(body, ensure_ascii=False)


def extract_seq_token(text: str) -> str | None:
    try:
        payload = json.loads(text)
    except Exception:
        return None
    token = payload.get("token")
    if not isinstance(token, str) or token.strip() == "":
        return None
    return token


def is_server_reply(text: str) -> bool:
    try:
        payload = json.loads(text)
    except Exception:
        return False
    return payload.get("cmd") == "pong"
