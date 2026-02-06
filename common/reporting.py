"""Shared reporting helpers with optional rich formatting."""

from __future__ import annotations

from typing import Any, Callable, Protocol, Sequence, TypeAlias, cast

Reporter = Callable[[str], None]
Renderable: TypeAlias = Any


class PanelPrinter(Protocol):
    def __call__(self, message: Renderable, title: str | None = None, style: str | None = None) -> None: ...


class TableBuilder(Protocol):
    def __call__(
        self,
        columns: Sequence[str],
        rows: Sequence[Sequence[str]],
        title: str | None = None,
        style: str | None = None,
    ) -> object: ...


def show_panel(paneler: PanelPrinter | None, message: Renderable, title: str | None = None, style: str | None = None) -> None:
    if paneler is None:
        return
    paneler(message, title, style)


def show_table(
    reporter: Reporter,
    paneler: PanelPrinter | None,
    table_builder: TableBuilder | None,
    *,
    title: str | None,
    columns: Sequence[str],
    rows: Sequence[Sequence[str]],
    style: str | None = None,
    wrap_panel: bool = True,
) -> None:
    if table_builder is None:
        lines = []
        if title:
            lines.append(f"{title}")
        header = " | ".join(columns)
        lines.append(header)
        lines.append("-" * len(header))
        for row in rows:
            lines.append(" | ".join(row))
        reporter("\n".join(lines))
        return

    table = table_builder(columns, rows, title=title, style=style)
    if paneler is None or not wrap_panel:
        reporter(str(table))
        return
    show_panel(paneler, table, title=title, style=style)


def _plain_reporter(message: str) -> None:
    if message.startswith("\r"):
        print(message, end="", flush=True)
        return
    print(message)


def _plain_panel(_message: str, _title: str | None = None, _style: str | None = None) -> None:
    return


def _has_markup(message: Renderable) -> bool:
    if not isinstance(message, str):
        return False
    return "[/" in message


def make_reporter(use_rich: bool | None = None) -> tuple[Reporter, PanelPrinter | None, TableBuilder | None]:
    if use_rich is False:
        return _plain_reporter, None, None

    try:
        from rich import box  # type: ignore
        from rich.console import Console  # type: ignore
        from rich.markup import escape  # type: ignore
        from rich.panel import Panel  # type: ignore
        from rich.table import Table  # type: ignore
    except Exception:
        return _plain_reporter, None, None

    console = Console()

    def reporter(message: str) -> None:
        if not isinstance(message, str):
            console.print(cast(Renderable, message))
            return
        if message.startswith("\r"):
            text = message if _has_markup(message) else escape(message)
            console.print(text, end="", soft_wrap=True)
            return
        text = message if _has_markup(message) else escape(message)
        console.print(text)

    def panel(message: Renderable, title: str | None = None, style: str | None = None) -> None:
        if isinstance(message, str):
            content = message if _has_markup(message) else escape(message)
        else:
            content = cast(Renderable, message)
        if style is None:
            console.print(Panel(content, title=title, box=box.ROUNDED))
        else:
            console.print(Panel(content, title=title, box=box.ROUNDED, style=style))

    def table_builder(
        columns: Sequence[str],
        rows: Sequence[Sequence[str]],
        title: str | None = None,
        style: str | None = None,
    ) -> object:
        table = Table(title=title, box=box.ROUNDED)
        if style:
            table.style = style
        for column in columns:
            table.add_column(column)
        for row in rows:
            table.add_row(*row)
        return table

    return reporter, panel, table_builder
