from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Callable

_RICH_TAG_RE = re.compile(r"\[/?[a-zA-Z0-9 _#=-]+\]")


def strip_rich_markup(text: str) -> str:
    return _RICH_TAG_RE.sub("", text)


def normalize_report_text(message: object) -> str:
    if message is None:
        return ""
    return strip_rich_markup(str(message))


@dataclass
class LogBuffer:
    lines: list[str] = field(default_factory=list)

    def clear(self) -> None:
        self.lines.clear()

    def append(self, raw_message: object) -> None:
        text = normalize_report_text(raw_message)
        if not text:
            return

        chunks = text.split("\n")
        for chunk in chunks:
            if chunk.startswith("\r"):
                replacement = chunk.lstrip("\r")
                if self.lines:
                    self.lines[-1] = replacement
                else:
                    self.lines.append(replacement)
                continue
            self.lines.append(chunk)

    def render(self, max_lines: int = 500) -> str:
        if max_lines <= 0:
            max_lines = 1
        trimmed = self.lines[-max_lines:]
        return "\n".join(trimmed)


def make_gui_reporter(emit: Callable[[str], None]) -> Callable[..., None]:
    def _report(message: str, end: str = "\n", flush: bool = False) -> None:  # noqa: ARG001
        if end == "":
            emit(message)
            return
        emit(f"{message}{end.rstrip()}")

    return _report
