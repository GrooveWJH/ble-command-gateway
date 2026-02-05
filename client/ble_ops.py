import asyncio
import json
from typing import Any, Callable, cast

from bleak import BleakClient, BleakScanner

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


def _filter_devices_by_name(devices: list[Any], target_name: str) -> list[Any]:
    if target_name:
        filtered = [d for d in devices if d.name and target_name in d.name]
    else:
        filtered = list(devices)
    filtered.sort(key=lambda d: (d.name or "", d.address))
    return filtered


async def discover_devices_with_progress(
    target_name: str,
    timeout: int,
    on_progress: Callable[[float, float, int, int], None] | None = None,
    refresh_interval: float = 0.1,
) -> tuple[list[Any], int]:
    seen: dict[str, Any] = {}

    def _on_detect(device: Any, _adv: Any) -> None:
        seen[device.address] = device

    scanner = BleakScanner(detection_callback=_on_detect)
    loop = asyncio.get_running_loop()
    start = loop.time()
    await scanner.start()
    try:
        while True:
            elapsed = loop.time() - start
            devices_now = list(seen.values())
            matched_now = _filter_devices_by_name(devices_now, target_name)
            if on_progress:
                on_progress(elapsed, float(timeout), len(devices_now), len(matched_now))
            if elapsed >= timeout:
                break
            await asyncio.sleep(refresh_interval)
    finally:
        await scanner.stop()

    devices = list(seen.values())
    filtered = _filter_devices_by_name(devices, target_name)
    return filtered, len(devices)


async def verify_target_service(client: BleakClient) -> str | None:
    services = client.services
    if services is None:
        get_services = getattr(client, "get_services", None)
        if callable(get_services):
            services = await cast(Any, get_services)()
        else:
            return "无法读取设备服务列表"

    service_uuids = {svc.uuid.lower() for svc in services}
    if SERVICE_UUID.lower() not in service_uuids:
        return f"未发现目标服务 UUID: {SERVICE_UUID}"

    service = services.get_service(SERVICE_UUID)
    if service is None:
        return f"无法读取目标服务: {SERVICE_UUID}"

    characteristic_uuids = {ch.uuid.lower() for ch in service.characteristics}
    missing = [
        uuid
        for uuid in (CHAR_WRITE_UUID, CHAR_READ_UUID)
        if uuid.lower() not in characteristic_uuids
    ]
    if missing:
        return f"目标设备缺少特征: {', '.join(missing)}"
    return None


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

            await client.write_gatt_char(CHAR_WRITE_UUID, payload, response=True)
            print("[发送] 配网参数已发送，等待终态...")
            return await wait_status(client, wait_timeout, verbose)
    except Exception as exc:  # noqa: BLE001
        return RunResult(ResultCode.FAILED, f"BLE 交互异常: {exc}")
