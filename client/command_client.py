import asyncio
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


def extract_final_result(code: str, status: str, data: dict[str, Any] | None) -> RunResult | None:
    if code == CODE_PROVISION_SUCCESS:
        ip = None if data is None else str(data.get("ip", "") or "")
        return RunResult(ResultCode.SUCCESS, "配网成功", ip or None)
    if code in {CODE_PROVISION_FAIL, CODE_BUSY}:
        return RunResult(ResultCode.FAILED, f"配网失败: {status}")
    if code == CODE_IN_PROGRESS:
        return None
    return None


async def discover_devices_with_progress(
    target_name: str,
    timeout: int,
    on_progress: Callable[[float, float, int, int], None] | None = None,
    refresh_interval: float = 0.1,
) -> tuple[list[Any], int]:
    result = await scan_devices(
        target_name=target_name,
        timeout=float(timeout),
        match_all=not target_name,
        refresh_interval=refresh_interval,
        stop_on_first_match=False,
        on_progress=on_progress,
    )
    filtered = filter_devices_by_name(result.devices, target_name, match_all=not target_name)
    return filtered, len(result.devices)


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


async def wait_response(client: BleakClient, request_id: str, timeout: int) -> CommandResponse | None:
    for _ in range(timeout):
        await asyncio.sleep(1)
        raw = await client.read_gatt_char(CHAR_READ_UUID)
        try:
            response = parse_response(raw)
        except CommandParseError:
            continue
        if response.request_id == request_id:
            return response
    return None


async def wait_status(client: BleakClient, request_id: str, timeout: int, verbose: bool = False) -> RunResult:
    for elapsed in range(1, timeout + 1):
        await asyncio.sleep(1)
        raw = await client.read_gatt_char(CHAR_READ_UUID)

        try:
            response = parse_response(raw)
        except CommandParseError as exc:
            status = f"<invalid response: {exc.message}>"
            print(f"\r[等待] {elapsed}/{timeout}s | 当前状态: {status[:60]}", end="", flush=True)
            continue

        status = response.text.strip() or "<empty>"
        if response.request_id != request_id:
            print(f"\r[等待] {elapsed}/{timeout}s | 当前状态: {status[:60]} (other id)", end="", flush=True)
            continue

        print(f"\r[等待] {elapsed}/{timeout}s | 当前状态: {status[:60]}", end="", flush=True)
        if verbose:
            print(f"\n[RESP] id={response.request_id} code={response.code} ok={response.ok} text={response.text}")

        final = bool((response.data or {}).get("final", False))
        final_result = extract_final_result(response.code, status, response.data)
        if final and final_result is not None:
            print()
            return final_result

    print()
    return RunResult(ResultCode.TIMEOUT, "等待超时，未收到成功/失败终态")


async def run_command(device: Any, command: str, args: dict[str, Any] | None, wait_timeout: int) -> RunResult:
    request = command_request(command, args or {})
    payload = encode_request(request)

    try:
        async with BleakClient(device) as client:
            verify_error = await verify_target_service(client)
            if verify_error:
                return RunResult(ResultCode.FAILED, verify_error)

            await write_with_fallback(client, CHAR_WRITE_UUID, payload)
            response = await wait_response(client, request.request_id, wait_timeout)
            if response is None:
                return RunResult(ResultCode.TIMEOUT, "等待命令响应超时")
            if response.ok:
                return RunResult(ResultCode.SUCCESS, response.text)
            return RunResult(ResultCode.FAILED, response.text)
    except Exception as exc:  # noqa: BLE001
        return RunResult(ResultCode.FAILED, f"BLE 交互异常: {exc}")


async def provision_device(
    device: Any,
    ssid: str,
    password: str,
    wait_timeout: int,
    verbose: bool,
) -> RunResult:
    request = command_request(CMD_PROVISION, {"ssid": ssid, "pwd": password})
    payload = encode_request(request)

    try:
        async with BleakClient(device) as client:
            print(f"[连接] {device.name} ({device.address})")
            verify_error = await verify_target_service(client)
            if verify_error:
                return RunResult(ResultCode.FAILED, verify_error)

            write_mode = await write_with_fallback(client, CHAR_WRITE_UUID, payload)
            print(f"[发送] 配网命令已发送（{write_mode}），等待终态...")
            return await wait_status(client, request.request_id, wait_timeout, verbose)
    except Exception as exc:  # noqa: BLE001
        return RunResult(ResultCode.FAILED, f"BLE 交互异常: {exc}")
