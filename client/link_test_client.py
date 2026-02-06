#!/usr/bin/env python3
"""Hello-world BLE link test client."""

from __future__ import annotations

import argparse
import asyncio
import logging
import time
import traceback
from typing import Any, Awaitable, TypeVar

from bleak import BleakClient

from client.ble_gatt import (
    WriteConfig,
    describe_service_shape_error,
    resolve_services,
    supports_write_without_response,
    verify_service_shape,
)
from ble.scan_transport import (
    Reporter,
    RuntimeState,
    find_target_device,
    make_disconnected_handler,
    run_step,
    summarize_exception,
)
from client.link_exchange import NotifyInbox, WriteSession, run_exchanges
from common.reporting import make_reporter, show_panel, show_table
from config.ble_uuid import CHAR_READ_UUID, CHAR_WRITE_UUID, SERVICE_UUID

T = TypeVar("T")


class Timer:
    def __init__(self, reporter: Reporter, paneler, table_builder) -> None:
        self.reporter = reporter
        self.paneler = paneler
        self.table_builder = table_builder
        self._starts: dict[str, float] = {}
        self._durations: dict[str, float] = {}

    def start(self, name: str) -> None:
        self._starts[name] = time.perf_counter()
        self.reporter(f"[client] timing start | {name}")

    def stop(self, name: str) -> None:
        start = self._starts.get(name)
        if start is None:
            return
        elapsed = time.perf_counter() - start
        self._durations[name] = elapsed
        self.reporter(f"[client] timing done  | {name} elapsed={elapsed:.2f}s")

    def report_summary(self) -> None:
        if not self._durations:
            return
        ordered = sorted(self._durations.items(), key=lambda item: item[1], reverse=True)
        rows = [[name, f"{elapsed:.2f}s"] for name, elapsed in ordered]
        show_table(
            self.reporter,
            self.paneler,
            self.table_builder,
            title="Timing Summary",
            columns=["Stage", "Elapsed"],
            rows=rows,
            style="cyan",
        )


async def run_step_logged(step: str, timeout: float, coro: Awaitable[T], reporter: Reporter) -> T:
    return await run_step(step, timeout, coro, reporter=reporter)


async def connect_and_exchange(
    runtime: RuntimeState,
    device: Any,
    args: argparse.Namespace,
    reporter: Reporter,
    timer: Timer,
) -> int:
    client = BleakClient(device, disconnected_callback=make_disconnected_handler(reporter))
    runtime.clients.add(client)

    inbox = NotifyInbox(reporter=reporter, paneler=runtime.paneler, table_builder=runtime.table_builder)
    notify_started = False

    try:
        reporter("[cyan][client] connecting...[/cyan]")
        timer.start("connect")
        await run_step_logged("connect", args.connect_timeout, client.connect(), reporter)
        timer.stop("connect")
        if not client.is_connected:
            return 5
        reporter(f"[green][client] connected={client.is_connected}[/green]")

        timer.start("discover services")
        services = await run_step_logged("discover services", args.op_timeout, resolve_services(client), reporter)
        timer.stop("discover services")
        if services is None:
            reporter("[client] unable to read GATT services")
            return 3
        verify_error = verify_service_shape(services, SERVICE_UUID, [CHAR_WRITE_UUID, CHAR_READ_UUID])
        if verify_error:
            reporter(f"[client] {describe_service_shape_error(verify_error, zh_cn=False)}")
            return 3

        service = services.get_service(SERVICE_UUID)
        characteristic_uuids = sorted(ch.uuid for ch in service.characteristics)
        show_table(
            reporter,
            runtime.paneler,
            runtime.table_builder,
            title="GATT Characteristics",
            columns=["UUID"],
            rows=[[uuid] for uuid in characteristic_uuids],
            style="cyan",
        )

        write_char = service.get_characteristic(CHAR_WRITE_UUID)

        write_props = list(write_char.properties)
        show_table(
            reporter,
            runtime.paneler,
            runtime.table_builder,
            title="Write Properties",
            columns=["Property"],
            rows=[[str(p).lower()] for p in write_props],
            style="cyan",
        )

        can_no_resp = supports_write_without_response(write_props)
        if args.allow_write_no_response and not can_no_resp:
            reporter("[client] write-without-response not advertised; force response=True mode")
        allow_no_resp = args.allow_write_no_response and can_no_resp

        timer.start("start notify")
        await run_step_logged(
            f"start notify {CHAR_READ_UUID}",
            args.op_timeout,
            client.start_notify(CHAR_READ_UUID, inbox.handler),
            reporter,
        )
        timer.stop("start notify")
        notify_started = True

        timer.start("exchanges")
        await run_exchanges(
            client=client,
            run_step=lambda step, timeout, coro: run_step_logged(step, timeout, coro, reporter),
            inbox=inbox,
            write_uuid=CHAR_WRITE_UUID,
            count=args.exchange_count,
            interval=args.exchange_interval,
            op_timeout=args.op_timeout,
            mode=args.exchange_mode,
            write_session=WriteSession(
                config=WriteConfig(allow_no_response=allow_no_resp),
                reporter=reporter,
                paneler=runtime.paneler,
                table_builder=runtime.table_builder,
            ),
            reporter=reporter,
            paneler=runtime.paneler,
            table_builder=runtime.table_builder,
        )
        timer.stop("exchanges")

        reporter(f"[client] link test success ({args.exchange_count}/{args.exchange_count} exchanges completed)")
        show_table(
            reporter,
            runtime.paneler,
            runtime.table_builder,
            title="Exchange Summary",
            columns=["Metric", "Value"],
            rows=[
                ["Mode", args.exchange_mode],
                ["Count", str(args.exchange_count)],
                ["Interval", f"{args.exchange_interval:.2f}s"],
                ["Result", "success"],
            ],
            style="green",
        )
        return 0
    finally:
        if notify_started and client.is_connected:
            try:
                timer.start("stop notify")
                await run_step_logged(
                    f"stop notify {CHAR_READ_UUID}",
                    args.op_timeout,
                    client.stop_notify(CHAR_READ_UUID),
                    reporter,
                )
                timer.stop("stop notify")
            except Exception as exc:  # noqa: BLE001
                reporter(f"[client] stop notify failed: {type(exc).__name__}: {exc}")

        if client.is_connected:
            try:
                reporter("[cyan][client] disconnecting...[/cyan]")
                timer.start("disconnect")
                await run_step_logged("disconnect", min(args.op_timeout, 5.0), client.disconnect(), reporter)
                timer.stop("disconnect")
                reporter("[green][client] disconnected[/green]")
            except Exception as exc:  # noqa: BLE001
                reporter(f"[client] disconnect failed: {type(exc).__name__}: {exc}")

        runtime.clients.discard(client)


async def run_once(runtime: RuntimeState, args: argparse.Namespace) -> int:
    reporter = runtime.reporter
    paneler = runtime.paneler
    timer = Timer(reporter, paneler, runtime.table_builder)
    timer.start("scan")
    device = await find_target_device(args.target_name, args.scan_timeout, runtime, reporter=reporter)
    timer.stop("scan")
    if device is None:
        timer.report_summary()
        return 2

    show_table(
        reporter,
        paneler,
        runtime.table_builder,
        title="Target Device",
        columns=["Field", "Value"],
        rows=[
            ["Name", str(getattr(device, "name", None))],
            ["Address", str(getattr(device, "address", None))],
        ],
        style="cyan",
    )

    for attempt in range(1, args.connect_retries + 1):
        reporter(f"[client] connecting... attempt={attempt}/{args.connect_retries}")

        try:
            result = await connect_and_exchange(runtime, device, args, reporter, timer)
            timer.report_summary()
            return result
        except Exception as exc:  # noqa: BLE001
            reporter(f"[client] BLE error (attempt {attempt}): {summarize_exception(exc)}")
            if args.full_traceback:
                reporter("[client] traceback:")
                reporter(traceback.format_exc().rstrip())
            if attempt < args.connect_retries:
                await asyncio.sleep(0.6 * attempt)

    timer.report_summary()
    return 5


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="BLE hello-world link client")
    parser.add_argument("--target-name", default="BLE_Hello_Server")
    parser.add_argument("--scan-timeout", type=int, default=20)
    parser.add_argument("--connect-retries", type=int, default=3)
    parser.add_argument("--connect-timeout", type=float, default=35.0)
    parser.add_argument("--op-timeout", type=float, default=15.0)
    parser.add_argument("--exchange-count", type=int, default=10)
    parser.add_argument("--exchange-interval", type=float, default=1.0)
    parser.add_argument("--exchange-mode", choices=["parallel", "sequential"], default="sequential")
    parser.add_argument("--allow-write-no-response", action="store_true")
    parser.add_argument("--full-traceback", action="store_true")
    return parser.parse_args()


async def run() -> int:
    logging.getLogger("bleak").setLevel(logging.DEBUG)
    reporter, paneler, table_builder = make_reporter()
    show_panel(paneler, "BLE link test client", "BLE Link Test", "cyan")
    runtime = RuntimeState(reporter=reporter, paneler=paneler, table_builder=table_builder)
    args = parse_args()
    try:
        return await run_once(runtime, args)
    finally:
        await runtime.cleanup()


def main() -> int:
    try:
        return asyncio.run(run())
    except KeyboardInterrupt:
        print("\n[client] interrupted")
        return 130
