from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor

from client.credential_store import load_cached_wifi_credentials
from client.gui.controller import GuiController
from client.gui.reporting import LogBuffer, make_gui_reporter
from client.gui.result_panel import ResultPanelPresenter
from client.gui.state import GuiState
from client.gui.view import EVENT_LOG, build_window
from client.library_api import SyncBleGatewayClient


def run_gui(args: argparse.Namespace) -> int:
    cached_ssid, cached_password = load_cached_wifi_credentials()
    state = GuiState(
        target_name=args.target_name,
        scan_timeout=args.scan_timeout,
        wait_timeout=args.wait_timeout,
        verbose=args.verbose,
        ssid=cached_ssid or "",
        password=cached_password or "",
    )

    window = build_window(state)

    def _emit_log(message: str) -> None:
        window.write_event_value(EVENT_LOG, message)

    gateway = SyncBleGatewayClient(
        target_name=state.target_name,
        scan_timeout=state.scan_timeout,
        wait_timeout=state.wait_timeout,
        verbose=state.verbose,
        reporter=make_gui_reporter(_emit_log),
    )
    executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="ble-gui-worker")
    controller = GuiController(
        window=window,
        state=state,
        gateway=gateway,
        executor=executor,
        log_buffer=LogBuffer(),
        presenter=ResultPanelPresenter(window),
    )
    return controller.run()
