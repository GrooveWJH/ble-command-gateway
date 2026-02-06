import asyncio
import time
from typing import Any, Callable

from bleak import BleakClient

from client.ble_gatt import (
    WriteConfig,
    WriteState,
    describe_service_shape_error,
    resolve_services,
    verify_service_shape,
    write_with_strategy,
)
from client.ble_scan import filter_devices_by_name, scan_devices
from client.models import ResultCode, RunResult
from config.ble_uuid import CHAR_READ_UUID, CHAR_WRITE_UUID, SERVICE_UUID
from protocol.command_ids import CMD_PROVISION
from protocol.envelope import (
    CODE_BUSY,
    CODE_IN_PROGRESS,
    CODE_PROVISION_FAIL,
    CODE_PROVISION_SUCCESS,
    CommandParseError,
    CommandResponse,
    command_request,
    encode_request,
    parse_response,
)


class _ChunkProgress:
    def __init__(self) -> None:
        self._progress: Any | None = None
        self._task_id: Any | None = None
        self._total: int | None = None
        self._rich_ready = False
        try:
            from rich.progress import BarColumn, Progress, TextColumn  # type: ignore
        except Exception:
            return

        self._progress = Progress(
            TextColumn("[green]接收分片[/green]"),
            BarColumn(
                bar_width=None,
                complete_style="green",
                finished_style="green",
                style="grey50",
            ),
            TextColumn("[green]{task.completed}/{task.total}[/green]"),
            TextColumn("[green]{task.fields[elapsed]}s[/green]"),
            expand=True,
            transient=True,
            refresh_per_second=20,
        )
        self._rich_ready = True

    @property
    def rich_enabled(self) -> bool:
        return self._rich_ready

    def update(self, received: int, total: int, elapsed_sec: int, reporter: Callable[..., None] | None) -> None:
        if not self._rich_ready or self._progress is None:
            if reporter:
                reporter(
                    f"\r[green][等待][/green] {elapsed_sec}s | 接收分片 {received}/{total}    ",
                    end="",
                    flush=True,
                )
            return

        if self._task_id is None or self._total != total:
            self._progress.start()
            self._task_id = self._progress.add_task("", total=total, completed=received, elapsed=elapsed_sec)
            self._total = total
            return

        self._progress.update(self._task_id, completed=received, elapsed=elapsed_sec)

    def stop(self) -> None:
        if self._progress is None:
            return
        try:
            self._progress.stop()
        except Exception:
            pass


def extract_final_result(code: str, status: str, data: dict[str, Any] | None) -> RunResult | None:
    if code == CODE_PROVISION_SUCCESS:
        ip = None if data is None else str(data.get("ip", "") or "")
        ssh_user = None if data is None else str(data.get("user", "") or "").strip()
        return RunResult(ResultCode.SUCCESS, "配网成功", ip or None, ssh_user or None)
    if code in {CODE_PROVISION_FAIL, CODE_BUSY}:
        return RunResult(ResultCode.FAILED, f"配网失败: {status}")
    if code == CODE_IN_PROGRESS:
        return None
    return None


async def discover_devices_with_progress(
    target_name: str,
    timeout: int,
    on_progress: Callable[[float, float, int, int], None] | None = None,
    on_detect: Callable[[Any], None] | None = None,
    refresh_interval: float = 0.1,
) -> tuple[list[Any], list[Any], int]:
    result = await scan_devices(
        target_name=target_name,
        timeout=float(timeout),
        match_all=not target_name,
        refresh_interval=refresh_interval,
        stop_on_first_match=True,
        on_progress=on_progress,
        on_detect=on_detect,
    )
    filtered = filter_devices_by_name(result.devices, target_name, match_all=not target_name)
    return result.devices, filtered, len(result.devices)


async def verify_target_service(client: BleakClient) -> str | None:
    services = await resolve_services(client)
    if services is None:
        return "无法读取设备服务列表"

    error = verify_service_shape(services, SERVICE_UUID, [CHAR_WRITE_UUID, CHAR_READ_UUID])
    if error is None:
        return None
    return describe_service_shape_error(error, zh_cn=True)


async def write_with_fallback(client: BleakClient, char_uuid: str, payload: bytes) -> str:
    outcome = await write_with_strategy(
        client,
        char_uuid,
        payload,
        config=WriteConfig(allow_no_response=True),
        state=WriteState(False),
    )
    return outcome.mode.value


async def wait_response(
    client: BleakClient,
    request_id: str,
    timeout: int,
    *,
    read_timeout: float = 5.0,
    reporter: Callable[..., None] | None = None,
) -> CommandResponse | None:
    chunks: dict[int, str] = {}
    total_chunks: int | None = None
    start = time.monotonic()
    last_chunk_time = start
    emitted_inline = False
    chunk_progress = _ChunkProgress()
    last_wait_note_second = -1
    saw_valid_frame = False

    def _elapsed() -> int:
        return int(time.monotonic() - start)

    def _print_wait(note: str, color: str = "yellow") -> None:
        nonlocal emitted_inline, last_wait_note_second
        if not reporter:
            return
        elapsed_sec = _elapsed()
        # Throttle noisy inline logs to one update per second.
        if elapsed_sec == last_wait_note_second:
            return
        last_wait_note_second = elapsed_sec
        reporter(
            f"\r[{color}][等待][/{color}] {elapsed_sec}s | {note}    ",
            end="",
            flush=True,
        )
        emitted_inline = True

    while True:
        if time.monotonic() - last_chunk_time > timeout:
            chunk_progress.stop()
            if reporter and emitted_inline:
                reporter("")  # newline
            return None

        await asyncio.sleep(0.2)
        try:
            raw = await asyncio.wait_for(client.read_gatt_char(CHAR_READ_UUID), timeout=read_timeout)
        except asyncio.TimeoutError:
            _print_wait("读取超时，继续等待...")
            continue
        try:
            response = parse_response(raw)
        except CommandParseError:
            if not saw_valid_frame:
                _print_wait("等待设备响应...")
            else:
                _print_wait("响应解析失败，继续等待...", color="red")
            continue
        saw_valid_frame = True
        if response.request_id != request_id:
            # BLE read characteristic keeps the previous payload until a new one is published.
            # Ignore payloads from other request IDs to avoid showing stale/irrelevant text.
            _print_wait("等待目标响应...")
            continue

        chunk = (response.data or {}).get("chunk")
        if isinstance(chunk, dict):
            idx = int(chunk.get("index", 0))
            total = int(chunk.get("total", 0))
            if idx > 0 and total > 0:
                if idx not in chunks:
                    chunks[idx] = response.text
                    last_chunk_time = time.monotonic()
                total_chunks = total
                chunk_progress.update(len(chunks), total, _elapsed(), reporter)
                if not chunk_progress.rich_enabled:
                    emitted_inline = True
                if total_chunks is not None and len(chunks) >= total_chunks:
                    assembled = "".join(chunks[i] for i in range(1, total_chunks + 1) if i in chunks)
                    data = dict(response.data or {})
                    data.pop("chunk", None)
                    chunk_progress.stop()
                    if reporter and emitted_inline:
                        reporter("")  # newline
                    return CommandResponse(
                        request_id=response.request_id,
                        ok=response.ok,
                        code=response.code,
                        text=assembled,
                        data=data,
                    )
                continue

        if reporter and emitted_inline:
            reporter("")  # newline
        chunk_progress.stop()
        return response


async def wait_status(client: BleakClient, request_id: str, timeout: int, verbose: bool = False) -> RunResult:
    read_errors = 0
    max_read_errors = 5
    idle_timeout = max(int(timeout), 1)
    hard_timeout = min(max(idle_timeout * 4, idle_timeout + 30), 180)
    start = time.monotonic()
    last_progress = start
    last_status = "waiting"
    last_signature: tuple[str, str, bool] | None = None

    def _compact_status(text: str, limit: int = 70) -> str:
        compact = text.replace("\n", " | ").strip()
        if len(compact) <= limit:
            return compact
        return f"{compact[:limit]}..."

    while True:
        now = time.monotonic()
        elapsed = int(now - start)
        idle_elapsed = int(now - last_progress)
        if elapsed >= hard_timeout:
            print()
            return RunResult(ResultCode.TIMEOUT, f"等待超时，未收到成功/失败终态（总时长>{hard_timeout}s）")
        if idle_elapsed >= idle_timeout:
            print()
            return RunResult(
                ResultCode.TIMEOUT,
                f"等待超时，连续 {idle_timeout}s 未收到新状态（最后状态: {_compact_status(last_status, 90)}）",
            )

        await asyncio.sleep(1)
        try:
            raw = await asyncio.wait_for(client.read_gatt_char(CHAR_READ_UUID), timeout=5.0)
        except asyncio.TimeoutError:
            print(
                f"\r[等待] {elapsed}s | 距上次更新 {idle_elapsed}s | 读取超时，继续等待...",
                end="",
                flush=True,
            )
            continue
        except Exception as exc:  # noqa: BLE001
            read_errors += 1
            err_text = f"{type(exc).__name__}: {exc}"
            print(
                f"\r[等待] {elapsed}s | 距上次更新 {idle_elapsed}s | "
                f"读取失败({read_errors}/{max_read_errors}): {err_text[:60]}",
                end="",
                flush=True,
            )
            if read_errors >= max_read_errors:
                print()
                return RunResult(ResultCode.FAILED, f"BLE 读取失败过多: {err_text}")
            continue
        else:
            read_errors = 0

        try:
            response = parse_response(raw)
        except CommandParseError as exc:
            status = f"<invalid response: {exc.message}>"
            print(f"\r[等待] {elapsed}s | 距上次更新 {idle_elapsed}s | 响应异常: {_compact_status(status)}", end="", flush=True)
            continue

        status = response.text.strip() or "<empty>"
        if response.request_id != request_id:
            print(
                f"\r[等待] {elapsed}s | 距上次更新 {idle_elapsed}s | 等待目标请求响应...",
                end="",
                flush=True,
            )
            continue

        final = bool((response.data or {}).get("final", False))
        signature = (response.code, status, final)
        is_new_update = signature != last_signature
        if is_new_update:
            last_signature = signature
            last_progress = time.monotonic()
            last_status = status
            print(f"\r[等待] {elapsed}s | 状态更新: {_compact_status(status)}", end="", flush=True)
        else:
            idle_elapsed = int(time.monotonic() - last_progress)
            print(
                f"\r[等待] {elapsed}s | 等待终态，距上次更新 {idle_elapsed}s | {_compact_status(status)}",
                end="",
                flush=True,
            )

        if verbose and is_new_update:
            print(f"\n[RESP] id={response.request_id} code={response.code} ok={response.ok} text={response.text}")

        final_result = extract_final_result(response.code, status, response.data)
        if final and final_result is not None:
            print()
            return final_result


async def run_command(
    device: Any,
    command: str,
    args: dict[str, Any] | None,
    wait_timeout: int,
    reporter: Callable[..., None] | None = None,
    client: BleakClient | None = None,
) -> RunResult:
    request = command_request(command, args or {})
    payload = encode_request(request)

    try:
        if reporter:
            reporter(f"[cyan][开始][/cyan] 执行命令 {command} ...")
        else:
            print(f"[开始] 执行命令 {command} ...")

        if client is None:
            async with BleakClient(device) as session_client:
                if reporter:
                    reporter(f"[cyan][连接][/cyan] {getattr(device, 'name', '<unknown>')} ({getattr(device, 'address', '<?>')})")
                else:
                    print(f"[连接] {getattr(device, 'name', '<unknown>')} ({getattr(device, 'address', '<?>')})")
                if reporter:
                    reporter("[cyan][校验][/cyan] 验证服务特征...")
                else:
                    print("[校验] 验证服务特征...")
                verify_error = await verify_target_service(session_client)
                if verify_error:
                    return RunResult(ResultCode.FAILED, verify_error)

                if reporter:
                    reporter(f"[cyan][发送][/cyan] 命令 {command} 已发送，等待响应...")
                else:
                    print(f"[发送] 命令 {command} 已发送，等待响应...")
                await write_with_fallback(session_client, CHAR_WRITE_UUID, payload)
                response = await wait_response(session_client, request.request_id, wait_timeout, reporter=reporter or print)
                if response is None:
                    return RunResult(ResultCode.TIMEOUT, "等待命令响应超时")
                if response.ok:
                    return RunResult(ResultCode.SUCCESS, response.text)
                return RunResult(ResultCode.FAILED, response.text)

        if not client.is_connected:
            if reporter:
                reporter(f"[cyan][连接][/cyan] {getattr(device, 'name', '<unknown>')} ({getattr(device, 'address', '<?>')})")
            else:
                print(f"[连接] {getattr(device, 'name', '<unknown>')} ({getattr(device, 'address', '<?>')})")
            await asyncio.wait_for(client.connect(), timeout=10.0)

        if reporter:
            reporter("[cyan][校验][/cyan] 验证服务特征...")
        else:
            print("[校验] 验证服务特征...")
        verify_error = await verify_target_service(client)
        if verify_error:
            return RunResult(ResultCode.FAILED, verify_error)

        if reporter:
            reporter(f"[cyan][发送][/cyan] 命令 {command} 已发送，等待响应...")
        else:
            print(f"[发送] 命令 {command} 已发送，等待响应...")
        await write_with_fallback(client, CHAR_WRITE_UUID, payload)
        response = await wait_response(client, request.request_id, wait_timeout, reporter=reporter or print)
        if response is None:
            return RunResult(ResultCode.TIMEOUT, "等待命令响应超时")
        if response.ok:
            return RunResult(ResultCode.SUCCESS, response.text)
        return RunResult(ResultCode.FAILED, response.text)
    except Exception as exc:  # noqa: BLE001
        message = str(exc).strip() or repr(exc)
        return RunResult(ResultCode.FAILED, f"BLE 交互异常: {type(exc).__name__}: {message}")


async def provision_device(
    device: Any,
    ssid: str,
    password: str,
    wait_timeout: int,
    verbose: bool,
    reporter: Callable[..., None] | None = None,
    client: BleakClient | None = None,
) -> RunResult:
    request = command_request(CMD_PROVISION, {"ssid": ssid, "pwd": password})
    payload = encode_request(request)

    try:
        if reporter:
            reporter(f"[cyan][开始][/cyan] 执行配网 SSID={ssid} ...")
        else:
            print(f"[开始] 执行配网 SSID={ssid} ...")

        if client is None:
            async with BleakClient(device) as session_client:
                if reporter:
                    reporter(f"[cyan][连接][/cyan] {device.name} ({device.address})")
                else:
                    print(f"[连接] {device.name} ({device.address})")
                if reporter:
                    reporter("[cyan][校验][/cyan] 验证服务特征...")
                else:
                    print("[校验] 验证服务特征...")
                verify_error = await verify_target_service(session_client)
                if verify_error:
                    return RunResult(ResultCode.FAILED, verify_error)

                write_mode = await write_with_fallback(session_client, CHAR_WRITE_UUID, payload)
                if reporter:
                    reporter(f"[cyan][发送][/cyan] 配网命令已发送（{write_mode}），等待终态...")
                else:
                    print(f"[发送] 配网命令已发送（{write_mode}），等待终态...")
                return await wait_status(session_client, request.request_id, wait_timeout, verbose)

        if not client.is_connected:
            if reporter:
                reporter(f"[cyan][连接][/cyan] {device.name} ({device.address})")
            else:
                print(f"[连接] {device.name} ({device.address})")
            await asyncio.wait_for(client.connect(), timeout=10.0)

        if reporter:
            reporter("[cyan][校验][/cyan] 验证服务特征...")
        else:
            print("[校验] 验证服务特征...")
        verify_error = await verify_target_service(client)
        if verify_error:
            return RunResult(ResultCode.FAILED, verify_error)

        write_mode = await write_with_fallback(client, CHAR_WRITE_UUID, payload)
        if reporter:
            reporter(f"[cyan][发送][/cyan] 配网命令已发送（{write_mode}），等待终态...")
        else:
            print(f"[发送] 配网命令已发送（{write_mode}），等待终态...")
        return await wait_status(client, request.request_id, wait_timeout, verbose)
    except Exception as exc:  # noqa: BLE001
        message = str(exc).strip() or repr(exc)
        return RunResult(ResultCode.FAILED, f"BLE 交互异常: {type(exc).__name__}: {message}")


async def open_device_session(
    device: Any,
    timeout: int,
    retries: int = 0,
    reporter: Callable[[str], None] | None = None,
) -> tuple[RunResult, BleakClient | None]:
    attempt = 0
    last_error: str | None = None
    total_attempts = max(1, retries + 1)
    while attempt < total_attempts:
        attempt += 1
        if reporter:
            reporter(f"[cyan][连接][/cyan] 尝试 {attempt}/{total_attempts} ...")
        client = BleakClient(device)
        try:
            await asyncio.wait_for(client.connect(), timeout=float(timeout))
            verify_error = await verify_target_service(client)
            if verify_error:
                await close_device_session(client)
                return RunResult(ResultCode.FAILED, verify_error), None
            return RunResult(ResultCode.SUCCESS, "已连接并验证服务"), client
        except Exception as exc:  # noqa: BLE001
            message = str(exc).strip() or repr(exc)
            last_error = f"{type(exc).__name__}: {message}"
            if reporter:
                reporter(f"[red][连接] 失败[/red]: {last_error}")
            await close_device_session(client)

    retry_text = "是" if retries > 0 else "否"
    return (
        RunResult(
            ResultCode.FAILED,
            f"连接失败: {last_error or 'unknown'} | 已尝试 {total_attempts} 次 | 是否重试: {retry_text}",
        ),
        None,
    )


async def close_device_session(client: BleakClient | None) -> None:
    if client is None:
        return
    try:
        if client.is_connected:
            await client.disconnect()
    except Exception:
        pass


async def probe_device(
    device: Any,
    timeout: int,
    retries: int = 0,
    reporter: Callable[[str], None] | None = None,
) -> RunResult:
    result, client = await open_device_session(device, timeout, retries=retries, reporter=reporter)
    await close_device_session(client)
    return result
