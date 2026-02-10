from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from client.library_api import SessionHandle
    from client.library_models import DeviceInfo


class ResultCode(int, Enum):
    SUCCESS = 0
    NOT_FOUND = 2
    FAILED = 3
    TIMEOUT = 4
    INPUT_ERROR = 5


@dataclass
class RunResult:
    code: ResultCode
    message: str
    ip: str | None = None
    ssh_user: str | None = None
    data: dict[str, Any] | None = None


@dataclass
class SessionState:
    target_name: str
    scan_timeout: int
    wait_timeout: int
    verbose: bool
    selected_device: DeviceInfo | None = None
    active_session: SessionHandle | None = None
    ssid: str | None = None
    password: str | None = None
    last_result: RunResult | None = None
