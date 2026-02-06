"""Unified execution runtime for registered commands."""

from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Awaitable, Callable

from protocol.envelope import CODE_INTERNAL_ERROR, CODE_TIMEOUT, CommandResponse, response_error

CommandCall = Callable[[], Awaitable[CommandResponse]]
Reporter = Callable[[str], None]


@dataclass(frozen=True)
class RuntimeConfig:
    logger: Reporter


class CommandRuntime:
    def __init__(self, config: RuntimeConfig) -> None:
        self._config = config

    async def run(self, request_id: str, timeout_sec: float, call: CommandCall) -> CommandResponse:
        try:
            return await asyncio.wait_for(call(), timeout=timeout_sec)
        except asyncio.TimeoutError:
            return response_error(request_id, CODE_TIMEOUT, f"Command timeout after {timeout_sec:.1f}s")
        except Exception as exc:  # noqa: BLE001
            self._config.logger(f"command execution error: {type(exc).__name__}: {exc}")
            return response_error(request_id, CODE_INTERNAL_ERROR, f"{type(exc).__name__}: {exc}")
