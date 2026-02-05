import math

from typing import Any

from client.models import ResultCode, RunResult, SessionState


def print_scan_progress(elapsed: float, total: float, total_devices: int, matched_devices: int) -> None:
    remaining = max(int(math.ceil(total - elapsed)), 0)
    print(
        f"\r[扫描] 倒计时 {remaining:>2}s | 总设备 {total_devices:>3} | 匹配 {matched_devices:>3}",
        end="",
        flush=True,
    )


def end_scan_progress() -> None:
    print()


def print_scan_no_match(total_count: int, target_name: str) -> None:
    if total_count == 0:
        print("未发现任何 BLE 设备。请确认目标设备已上电并处于可发现状态。")
    else:
        print(f"共发现 {total_count} 个 BLE 设备，但没有匹配过滤名 '{target_name}'。")
        print("可尝试：修改过滤名、留空过滤名，或增大扫描时间。")


def show_state(state: SessionState) -> None:
    print("\n=== 会话状态 ===")
    print(f"过滤名: {state.target_name}")
    print(f"扫描超时: {state.scan_timeout}s")
    print(f"等待超时: {state.wait_timeout}s")
    print(
        f"当前设备: {getattr(state.selected_device, 'name', None)} / "
        f"{getattr(state.selected_device, 'address', None)}"
    )
    print(f"Wi-Fi SSID: {state.ssid}")
    print(f"Wi-Fi 密码: {'***' if state.password else None}")
    if state.last_result:
        print(f"上次结果: {state.last_result.code.name} - {state.last_result.message}")
        if state.last_result.ip:
            print(f"上次IP: {state.last_result.ip}")
    print("================\n")


def print_final(result: RunResult) -> None:
    state = "SUCCESS" if result.code is ResultCode.SUCCESS else "FAILED"
    print(f"\n=== 结果: {state} ===")
    print(result.message)
    if result.ip:
        print(f"Server IP: {result.ip}")
        print(f"SSH: ssh nvidia@{result.ip}")


def build_device_choices(devices: list[Any]) -> list[dict[str, Any]]:
    return [{"value": d, "name": f"{d.name or '<NoName>'}  |  {d.address}"} for d in devices]
