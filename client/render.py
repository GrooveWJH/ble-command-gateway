import math
import json
import re

from typing import Any, Callable

from client.models import ResultCode, RunResult, SessionState


def _get_console() -> Any | None:
    try:
        from rich.console import Console  # type: ignore
    except Exception:
        return None
    return Console()


def print_note(message: str) -> None:
    console = _get_console()
    if console is None:
        print(message)
        return
    console.print(f"[cyan]{message}[/cyan]")


def print_scan_progress(
    elapsed: float,
    total: float,
    total_devices: int,
    matched_devices: int,
    last_seen: str | None = None,
) -> None:
    remaining = max(int(math.ceil(total - elapsed)), 0)
    suffix = ""
    if last_seen:
        trimmed = last_seen
        if len(trimmed) > 40:
            trimmed = trimmed[:37] + "..."
        suffix = f" | {trimmed}"
    print(
        f"\r[扫描] 倒计时 {remaining:>2}s | 总设备 {total_devices:>3} | 匹配 {matched_devices:>3}{suffix}",
        end="",
        flush=True,
    )


def make_ui_reporter() -> Callable[..., None]:
    try:
        from rich.console import Console  # type: ignore
    except Exception:
        def _plain(message: str, end: str = "\n", flush: bool = False) -> None:
            print(message, end=end, flush=flush)

        return _plain

    console = Console()
    tag_re = re.compile(r"\[/?[a-zA-Z0-9 _-]+\]")

    def _report(message: str, end: str = "\n", flush: bool = False) -> None:
        # For inline progress updates, avoid rich markup to prevent garbled output.
        if message.startswith("\r") or end == "":
            text = tag_re.sub("", message)
            console.file.write(text)
            if flush:
                console.file.flush()
            return
        console.print(message, end=end, soft_wrap=True, highlight=False)

    return _report


def end_scan_progress() -> None:
    print()


def print_scan_no_match(total_count: int, target_name: str) -> None:
    console = _get_console()
    if total_count == 0:
        if console is None:
            print("未发现任何 BLE 设备。请确认目标设备已上电并处于可发现状态。")
            return
        from rich.panel import Panel  # type: ignore

        console.print(
            Panel(
                "未发现任何 BLE 设备。请确认目标设备已上电并处于可发现状态。",
                title="[bold yellow]SCAN RESULT[/bold yellow]",
                border_style="yellow",
                expand=False,
            )
        )
        return

    message = (
        f"共发现 {total_count} 个 BLE 设备，但没有匹配过滤名 '{target_name}'。\n"
        "可尝试：修改过滤名、留空过滤名，或增大扫描时间。"
    )
    if console is None:
        print(message)
        return

    from rich.panel import Panel  # type: ignore

    console.print(
        Panel(
            message,
            title="[bold yellow]SCAN RESULT[/bold yellow]",
            border_style="yellow",
            expand=False,
        )
    )


def show_state(state: SessionState) -> None:
    console = _get_console()
    device_name = getattr(state.selected_device, "name", None)
    device_addr = getattr(state.selected_device, "address", None)
    if console is None:
        print("=== 会话状态 ===")
        print(f"过滤名: {state.target_name}")
        print(f"扫描超时: {state.scan_timeout}s")
        print(f"等待超时: {state.wait_timeout}s")
        print(f"当前设备: {device_name} / {device_addr}")
        print(f"Wi-Fi SSID: {state.ssid}")
        print(f"Wi-Fi 密码: {'***' if state.password else None}")
        if state.last_result:
            print(f"上次结果: {state.last_result.code.name} - {state.last_result.message}")
            if state.last_result.ip:
                print(f"上次IP: {state.last_result.ip}")
        print("================")
        return

    from rich.panel import Panel  # type: ignore
    from rich.table import Table  # type: ignore

    table = Table(show_header=False, box=None, pad_edge=False)
    table.add_column("key", style="cyan", no_wrap=True)
    table.add_column("value")
    table.add_row("过滤名", state.target_name)
    table.add_row("扫描超时", f"{state.scan_timeout}s")
    table.add_row("等待超时", f"{state.wait_timeout}s")
    table.add_row("当前设备", f"{device_name} / {device_addr}")
    table.add_row("Wi-Fi SSID", str(state.ssid))
    table.add_row("Wi-Fi 密码", "***" if state.password else "None")
    if state.last_result:
        table.add_row("上次结果", f"{state.last_result.code.name} - {state.last_result.message}")
        if state.last_result.ip:
            table.add_row("上次IP", state.last_result.ip)

    console.print(
        Panel(
            table,
            title="[bold cyan]SESSION STATE[/bold cyan]",
            border_style="cyan",
            expand=False,
        )
    )


def print_final(result: RunResult) -> None:
    state = "SUCCESS" if result.code is ResultCode.SUCCESS else "FAILED"
    body_lines = [result.message]
    if result.ip:
        body_lines.append(f"Server IP: {result.ip}")
        ssh_user = result.ssh_user or "username"
        body_lines.append(f"SSH: ssh {ssh_user}@{result.ip}")
    body = "\n".join(body_lines)

    try:
        from rich.console import Console  # type: ignore
        from rich.panel import Panel  # type: ignore
    except Exception:
        print(f"=== 结果: {state} ===")
        print(body)
        print("-" * 48)
        return

    console = Console()
    if result.code is ResultCode.SUCCESS:
        title = "[bold green]RESULT: SUCCESS[/bold green]"
        border_style = "green"
    elif result.code is ResultCode.TIMEOUT:
        title = "[bold yellow]RESULT: TIMEOUT[/bold yellow]"
        border_style = "yellow"
    else:
        title = "[bold red]RESULT: FAILED[/bold red]"
        border_style = "red"
    console.print(Panel(body, title=title, border_style=border_style, expand=True))


def build_device_choices(devices: list[Any]) -> list[dict[str, Any]]:
    choices: list[dict[str, Any]] = []
    for device in devices:
        name = getattr(device, "adv_name", None) or getattr(device, "name", None) or "<NoName>"
        addr = getattr(device, "address", None) or "<?>"
        uuids = getattr(device, "adv_uuids", None) or []
        uuid_text = ",".join(uuids) if uuids else "-"
        choices.append({"value": device, "name": f"{name}  |  {addr}  |  {uuid_text}"})
    return choices


def print_named_devices(devices: list[Any]) -> None:
    named = [d for d in devices if (getattr(d, "adv_name", None) or getattr(d, "name", None))]
    if not named:
        return

    console = _get_console()
    if console is None:
        print("已发现有名字的设备：")
        for device in named:
            name = getattr(device, "adv_name", None) or getattr(device, "name", None) or "<NoName>"
            addr = getattr(device, "address", None) or "<?>"
            uuids = getattr(device, "adv_uuids", None) or []
            uuid_text = ",".join(uuids) if uuids else "-"
            print(f"- {name}  |  {addr}  |  {uuid_text}")
        return

    from rich.table import Table  # type: ignore

    table = Table(title="[bold cyan]已发现有名字的设备[/bold cyan]", expand=True)
    table.add_column("Name", style="green")
    table.add_column("Address", style="cyan")
    table.add_column("UUID(s)", overflow="fold")
    for device in named:
        name = getattr(device, "adv_name", None) or getattr(device, "name", None) or "<NoName>"
        addr = getattr(device, "address", None) or "<?>"
        uuids = getattr(device, "adv_uuids", None) or []
        uuid_text = ",".join(uuids) if uuids else "-"
        table.add_row(name, addr, uuid_text)
    console.print(table)


def print_wifi_scan_result(raw_json: str) -> bool:
    try:
        payload = json.loads(raw_json)
    except Exception:
        return False
    if not isinstance(payload, dict):
        return False
    aps = payload.get("aps")
    if not isinstance(aps, list):
        return False

    console = _get_console()
    if console is None:
        print(f"Found {len(aps)} AP(s):")
        for row in aps:
            if not isinstance(row, dict):
                continue
            ssid = str(row.get("ssid", "")).strip()
            signal = int(row.get("signal", 0) or 0)
            chan = str(row.get("chan", "-")).strip() or "-"
            print(f"- {ssid} | SIGNAL={signal}% | CH={chan}")
        return True

    from rich.panel import Panel  # type: ignore
    from rich.table import Table  # type: ignore

    table = Table(expand=True)
    table.add_column("SSID", style="cyan")
    table.add_column("CH", justify="right", style="white")
    table.add_column("SIGNAL", justify="right")

    for row in aps:
        if not isinstance(row, dict):
            continue
        ssid = str(row.get("ssid", "")).strip()
        chan = str(row.get("chan", "-")).strip() or "-"
        try:
            signal = int(row.get("signal", 0) or 0)
        except Exception:
            signal = 0
        signal = max(0, min(100, signal))
        if signal >= 70:
            signal_text = f"[green]{signal}%[/green]"
        elif signal >= 40:
            signal_text = f"[yellow]{signal}%[/yellow]"
        else:
            signal_text = f"[red]{signal}%[/red]"
        table.add_row(ssid, chan, signal_text)

    count = int(payload.get("count", len(aps)) or len(aps))
    console.print(
        Panel(
            table,
            title=f"[bold green]RESULT: SUCCESS[/bold green]  Wi-Fi 扫描结果 ({count} AP)",
            border_style="green",
            expand=True,
        )
    )
    return True
