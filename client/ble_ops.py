import asyncio
import json
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
from config import (
    CHAR_READ_UUID,
    CHAR_WRITE_UUID,
    PASSWORD_KEY,
    SERVICE_UUID,
    SSID_KEY,
    STATUS_BUSY_PREFIX,
    STATUS_FAIL_PREFIX,
    STATUS_SUCCESS_PREFIX,
)
from client.models import ResultCode, RunResult


def extract_final_result(status: str) -> RunResult | None:
    if status.startswith(STATUS_SUCCESS_PREFIX):
        return RunResult(ResultCode.SUCCESS, "配网成功", status.split(":", 1)[1])
    if status.startswith(STATUS_FAIL_PREFIX):
        return RunResult(ResultCode.FAILED, f"配网失败: {status}")
    if status.startswith(STATUS_BUSY_PREFIX):
        return RunResult(ResultCode.FAILED, f"设备忙: {status}")
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


async def wait_status(client: BleakClient, timeout: int, verbose: bool = False) -> RunResult:
    last_status = ""
    for elapsed in range(1, timeout + 1):
        await asyncio.sleep(1)
        raw = await client.read_gatt_char(CHAR_READ_UUID)
        status = bytes(raw).decode("utf-8", errors="ignore").strip() or "<empty>"

        print(f"\r[等待] {elapsed}/{timeout}s | 当前状态: {status[:60]}", end="", flush=True)
        if verbose and status != last_status:
            print(f"\n[STATUS] {status}")
        last_status = status

        final_result = extract_final_result(status)
        if final_result is not None:
            print()
            return final_result

    print()
    return RunResult(ResultCode.TIMEOUT, "等待超时，未收到成功/失败终态")


async def provision_device(
    device: Any,
    ssid: str,
    password: str,
    wait_timeout: int,
    verbose: bool,
) -> RunResult:
    payload = json.dumps({SSID_KEY: ssid, PASSWORD_KEY: password}, ensure_ascii=False).encode("utf-8")

    try:
        async with BleakClient(device) as client:
            print(f"[连接] {device.name} ({device.address})")
            verify_error = await verify_target_service(client)
            if verify_error:
                return RunResult(ResultCode.FAILED, verify_error)

            write_mode = await write_with_fallback(client, CHAR_WRITE_UUID, payload)
            print(f"[发送] 配网参数已发送（{write_mode}），等待终态...")
            return await wait_status(client, wait_timeout, verbose)
    except Exception as exc:  # noqa: BLE001
        return RunResult(ResultCode.FAILED, f"BLE 交互异常: {exc}")
