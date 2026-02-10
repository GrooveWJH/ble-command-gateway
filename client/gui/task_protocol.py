from __future__ import annotations

from concurrent.futures import Future
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

from client.gui.state import TaskResult


class GuiTaskKind(str, Enum):
    SCAN = "scan"
    CONNECT = "connect"
    DISCONNECT = "disconnect"
    PROVISION = "provision"
    DIAGNOSTIC = "diagnostic"
    HEARTBEAT = "heartbeat"


@dataclass(frozen=True)
class GuiTaskRequest:
    kind: GuiTaskKind
    affects_busy: bool = True
    meta: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True)
class GuiTaskDone:
    request: GuiTaskRequest
    future: Future[TaskResult]

    @property
    def kind(self) -> GuiTaskKind:
        return self.request.kind

    @property
    def affects_busy(self) -> bool:
        return self.request.affects_busy

    @property
    def meta(self) -> dict[str, Any]:
        return self.request.meta
