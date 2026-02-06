"""Declarative command registry and dispatcher."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Awaitable, Callable

from commands.runtime import CommandRuntime, RuntimeConfig
from commands.schemas import CommandSpec
from protocol.envelope import CODE_BAD_REQUEST, CODE_UNKNOWN_COMMAND, CommandRequest, CommandResponse, response_error

CommandHandler = Callable[["DispatchContext", CommandRequest], Awaitable[CommandResponse]]
SystemRunner = Callable[[str, str | None, float], Awaitable[tuple[bool, str]]]


@dataclass(frozen=True)
class DispatchContext:
    read_status_text: Callable[[], str]
    start_provision: Callable[[str, str, str], Awaitable[None]]
    start_shutdown: Callable[[str], Awaitable[None]]
    run_system_command: SystemRunner


@dataclass(frozen=True)
class RegisteredCommand:
    spec: CommandSpec
    handler: CommandHandler


class CommandDispatcher:
    def __init__(self, context: DispatchContext, logger: Callable[[str], None]) -> None:
        self._context = context
        self._registry: dict[str, RegisteredCommand] = {}
        self._runtime = CommandRuntime(RuntimeConfig(logger=logger))

    def register(self, spec: CommandSpec, handler: CommandHandler) -> None:
        if spec.name in self._registry:
            raise ValueError(f"duplicate command: {spec.name}")
        self._registry[spec.name] = RegisteredCommand(spec=spec, handler=handler)

    async def dispatch(self, request: CommandRequest) -> CommandResponse:
        registered = self._registry.get(request.command)
        if registered is None:
            return response_error(request.request_id, CODE_UNKNOWN_COMMAND, f"Unknown command: {request.command}")

        validation_error = registered.spec.validate_args(request.args)
        if validation_error is not None:
            return response_error(request.request_id, CODE_BAD_REQUEST, validation_error)

        return await self._runtime.run(
            request.request_id,
            registered.spec.timeout_sec,
            lambda: registered.handler(self._context, request),
        )

    def render_help(self, target_cmd: str | None = None) -> str:
        names = sorted(self._registry.keys())
        if target_cmd:
            reg = self._registry.get(target_cmd)
            if reg is None:
                return f"Unknown command: {target_cmd}"
            return self._render_single_help(reg.spec)

        lines = ["Available commands:"]
        for name in names:
            lines.append(f"- {name}")
        lines.append("")
        lines.append("Use help with args.cmd for details, e.g. {\"cmd\":\"provision\"}.")
        return "\n".join(lines)

    def list_commands(self) -> list[CommandSpec]:
        return [item.spec for item in self._registry.values()]

    @staticmethod
    def _render_single_help(spec: CommandSpec) -> str:
        lines = [
            f"Command: {spec.name}",
            f"Summary: {spec.summary}",
            f"Usage: {spec.usage}",
            f"Permission: {spec.permission}",
            f"Risk: {spec.risk}",
            f"Timeout: {spec.timeout_sec:.1f}s",
        ]
        if spec.args:
            lines.append("Args:")
            for arg in spec.args:
                required = "required" if arg.required else "optional"
                lines.append(f"- {arg.name} ({arg.type_name}, {required}): {arg.description}")
        return "\n".join(lines)
