from dataclasses import dataclass
from enum import Enum
from typing import Any


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


@dataclass
class SessionState:
    target_name: str
    scan_timeout: int
    wait_timeout: int
    verbose: bool
    selected_device: Any | None = None
    ssid: str | None = None
    password: str | None = None
    last_result: RunResult | None = None
