"""Shared helpers for hello-world BLE link test messages."""

from __future__ import annotations

import re
from typing import Final

CLIENT_HELLO_PREFIX: Final = "hello from client_link_test.py"
SERVER_HELLO_PREFIX: Final = "hello from server_link_test.py"

_SEQ_RE = re.compile(r"\bseq=(\d{1,4})/(\d{1,4})\b")


def build_seq_token(seq: int, total: int) -> str:
    if total <= 0:
        raise ValueError("total must be > 0")
    if seq <= 0:
        raise ValueError("seq must be > 0")
    width = max(2, len(str(total)))
    return f"seq={seq:0{width}d}/{total:0{width}d}"


def build_client_payload(seq: int, total: int, prefix: str = CLIENT_HELLO_PREFIX) -> str:
    return f"{prefix} | {build_seq_token(seq, total)}"


def build_server_reply(received_text: str, prefix: str = SERVER_HELLO_PREFIX) -> str:
    return f"{prefix} | got: {received_text}"


def extract_seq_token(text: str) -> str | None:
    match = _SEQ_RE.search(text)
    if match is None:
        return None
    seq = int(match.group(1))
    total = int(match.group(2))
    return build_seq_token(seq, total)


def is_server_reply(text: str) -> bool:
    return SERVER_HELLO_PREFIX in text
