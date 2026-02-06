"""Schema for declarative command registration."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class CommandArgSpec:
    name: str
    type_name: str = "str"
    required: bool = False
    description: str = ""


@dataclass(frozen=True)
class CommandSpec:
    name: str
    summary: str
    usage: str
    permission: str = "user"
    risk: str = "low"
    timeout_sec: float = 5.0
    args: tuple[CommandArgSpec, ...] = field(default_factory=tuple)

    def validate_args(self, payload: dict[str, Any]) -> str | None:
        for arg in self.args:
            value = payload.get(arg.name)
            if arg.required and (value is None or (isinstance(value, str) and value.strip() == "")):
                return f"field `args.{arg.name}` is required"
            if value is None:
                continue
            if arg.type_name == "str" and not isinstance(value, str):
                return f"field `args.{arg.name}` must be string"
        return None
