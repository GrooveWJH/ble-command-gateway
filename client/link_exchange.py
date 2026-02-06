"""Message exchange helpers for hello-world BLE link tests."""

from __future__ import annotations

import asyncio
from collections import defaultdict, deque
from dataclasses import dataclass, field
from typing import Any, Awaitable, Callable, TypeVar

from bleak import BleakClient

from client.ble_gatt import WriteConfig, WriteOutcome, WriteState, write_with_strategy
from common.reporting import PanelPrinter, TableBuilder, show_table
from protocol.link import build_client_payload, build_seq_token, extract_seq_token, is_server_reply

T = TypeVar("T")
StepRunner = Callable[[str, float, Awaitable[T]], Awaitable[T]]
Reporter = Callable[[str], None]


def _show_exchange_table(
    reporter: Reporter,
    paneler: PanelPrinter | None,
    table_builder: TableBuilder | None,
    *,
    title: str,
    rows: list[list[str]],
    style: str = "cyan",
) -> None:
    show_table(
        reporter,
        paneler,
        table_builder,
        title=title,
        columns=["Dir", "Seq", "Mode", "Bytes", "Text"],
        rows=rows,
        style=style,
        wrap_panel=False,
    )


@dataclass
class WriteSession:
    config: WriteConfig
    state: WriteState = field(default_factory=WriteState)
    reporter: Reporter = print
    paneler: PanelPrinter | None = None
    table_builder: TableBuilder | None = None

    async def write(self, client: BleakClient, write_uuid: str, payload: bytes) -> WriteOutcome:
        outcome = await write_with_strategy(
            client,
            write_uuid,
            payload,
            config=self.config,
            state=self.state,
        )
        self.state = outcome.next_state
        if outcome.detail:
            self.reporter(f"[client] write(response=True) failed: {outcome.detail}")
        return outcome


class NotifyInbox:
    def __init__(
        self,
        reporter: Reporter = print,
        paneler: PanelPrinter | None = None,
        table_builder: TableBuilder | None = None,
    ) -> None:
        self._queue: asyncio.Queue[str] = asyncio.Queue()
        self._pending: dict[str, deque[str]] = defaultdict(deque)
        self.reporter = reporter
        self.paneler = paneler
        self.table_builder = table_builder

    def handler(self, _: Any, data: bytearray) -> None:
        text = bytes(data).decode("utf-8", errors="ignore").strip() or "<empty>"
        self._queue.put_nowait(text)

    async def wait_for_token(self, token: str) -> str:
        pending = self._pending[token]
        if pending:
            return pending.popleft()

        while True:
            text = await self._queue.get()
            if not is_server_reply(text):
                self.reporter(f"[client] notify ignored (not server reply): {text}")
                continue

            current = extract_seq_token(text)
            if current is None:
                self.reporter(f"[client] notify ignored (missing seq token): {text}")
                continue

            if current == token:
                return text

            self._pending[current].append(text)
            self.reporter(f"[client] notify buffered token={current} while waiting={token}")


async def run_exchanges(
    client: BleakClient,
    run_step: StepRunner,
    inbox: NotifyInbox,
    write_uuid: str,
    count: int,
    interval: float,
    op_timeout: float,
    mode: str,
    write_session: WriteSession,
    reporter: Reporter = print,
    paneler: PanelPrinter | None = None,
    table_builder: TableBuilder | None = None,
) -> None:
    async def send_one(index: int) -> None:
        text = build_client_payload(index, count)
        outcome = await run_step(
            f"write[{index}/{count}] {write_uuid}",
            op_timeout,
            write_session.write(client, write_uuid, text.encode("utf-8")),
        )
        _show_exchange_table(
            reporter,
            paneler,
            table_builder,
            title="Client Send",
            rows=[
                [
                    "-> send",
                    f"{index:02d}/{count:02d}",
                    outcome.mode.value,
                    str(len(text.encode("utf-8"))),
                    text,
                ]
            ],
        )

    async def recv_one(index: int) -> None:
        token = build_seq_token(index, count)
        reply = await run_step(
            f"wait notify[{index}/{count}]",
            op_timeout,
            inbox.wait_for_token(token),
        )
        _show_exchange_table(
            reporter,
            paneler,
            table_builder,
            title="Client Recv",
            rows=[
                [
                    "<- recv",
                    f"{index:02d}/{count:02d}",
                    "-",
                    str(len(reply.encode("utf-8"))),
                    reply,
                ]
            ],
        )

    async def send_loop() -> None:
        for i in range(1, count + 1):
            await send_one(i)
            if i < count:
                await asyncio.sleep(interval)

    async def recv_loop() -> None:
        for i in range(1, count + 1):
            await recv_one(i)

    reporter(f"[client] exchange mode={mode} count={count} interval={interval:.2f}s")
    if mode == "sequential":
        for i in range(1, count + 1):
            await send_one(i)
            await recv_one(i)
            if i < count:
                await asyncio.sleep(interval)
        return

    reporter("[client] parallel mode enabled: notifications are buffered by seq token")
    async with asyncio.TaskGroup() as tg:
        tg.create_task(recv_loop())
        tg.create_task(send_loop())
