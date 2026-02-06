import argparse
import asyncio

from protocol.command_ids import CMD_HELP, CMD_NET_IFCONFIG, CMD_SHUTDOWN, CMD_STATUS, CMD_SYS_WHOAMI
from config.defaults import DEFAULT_DEVICE_NAME, DEFAULT_SCAN_TIMEOUT, DEFAULT_WAIT_TIMEOUT
from client.command_client import discover_devices_with_progress, provision_device, run_command
from client.models import ResultCode, RunResult, SessionState
from client.prompting import ask_list, ask_secret, ask_text
from client.render import (
    build_device_choices,
    end_scan_progress,
    print_final,
    print_scan_no_match,
    print_scan_progress,
    show_state,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Interactive BLE Wi-Fi provisioning client (InquirerPy)",
    )
    parser.add_argument("--target-name", default=DEFAULT_DEVICE_NAME, help="BLE device name contains this string")
    parser.add_argument("--scan-timeout", type=int, default=DEFAULT_SCAN_TIMEOUT, help="BLE scan timeout seconds")
    parser.add_argument("--wait-timeout", type=int, default=DEFAULT_WAIT_TIMEOUT, help="Wait status timeout seconds")
    parser.add_argument("--verbose", action="store_true", help="Print all status polling logs")
    return parser.parse_args()


def choose_action() -> str:
    return ask_list(
        "请选择操作",
        choices=[
            {"value": "scan", "name": "扫描并选择设备"},
            {"value": "set_target", "name": "修改设备名过滤条件"},
            {"value": "set_wifi", "name": "设置 Wi-Fi 凭据"},
            {"value": "provision", "name": "执行配网（当前选中设备）"},
            {"value": "device_help", "name": "查看设备 help"},
            {"value": "device_status", "name": "查看设备 status"},
            {"value": "device_whoami", "name": "查看设备 whoami"},
            {"value": "device_ifconfig", "name": "查看设备 ifconfig"},
            {"value": "device_shutdown", "name": "执行设备 shutdown"},
            {"value": "one_shot", "name": "一键流程（扫描 -> 输入 -> 配网）"},
            {"value": "show", "name": "查看当前会话状态"},
            {"value": "exit", "name": "退出"},
        ],
        default="scan",
    )


def prompt_target_name(current: str) -> str:
    value = ask_text("设备名过滤（包含匹配）", default=current).strip()
    return value or current


def prompt_wifi(current_ssid: str | None, current_password: str | None) -> tuple[str, str]:
    ssid = ask_text("Wi-Fi SSID", default=current_ssid or "").strip()
    if not ssid:
        raise ValueError("SSID 不能为空")

    password = ask_secret("Wi-Fi 密码（可留空）", default=current_password or "")
    return ssid, password


def scan_with_feedback(state: SessionState) -> tuple[list[object], int]:
    def _on_progress(elapsed: float, total: float, total_devices: int, matched_devices: int) -> None:
        print_scan_progress(elapsed, total, total_devices, matched_devices)

    try:
        return asyncio.run(
            discover_devices_with_progress(
                target_name=state.target_name,
                timeout=state.scan_timeout,
                on_progress=_on_progress,
            )
        )
    finally:
        end_scan_progress()


def select_device_interactive(devices: list[object], total_count: int, target_name: str) -> object | None:
    if not devices:
        print_scan_no_match(total_count, target_name)
        return None

    choices = build_device_choices(devices)
    return ask_list("选择目标设备", choices=choices)


def run_interactive(args: argparse.Namespace) -> int:
    state = SessionState(
        target_name=args.target_name,
        scan_timeout=args.scan_timeout,
        wait_timeout=args.wait_timeout,
        verbose=args.verbose,
    )

    print("BLE 配网交互模式已启动（InquirerPy）")

    while True:
        action = choose_action()

        if action == "exit":
            return int(state.last_result.code) if state.last_result else 0

        if action == "set_target":
            state.target_name = prompt_target_name(state.target_name)
            continue

        if action == "set_wifi":
            try:
                state.ssid, state.password = prompt_wifi(state.ssid, state.password)
            except ValueError as exc:
                print(exc)
            continue

        if action == "scan":
            devices, total_count = scan_with_feedback(state)
            state.selected_device = select_device_interactive(devices, total_count, state.target_name)
            continue

        if action == "show":
            show_state(state)
            continue

        if action == "one_shot":
            devices, total_count = scan_with_feedback(state)
            state.selected_device = select_device_interactive(devices, total_count, state.target_name)
            if state.selected_device is None:
                state.last_result = RunResult(ResultCode.NOT_FOUND, "未发现匹配设备")
                print_final(state.last_result)
                continue
            try:
                state.ssid, state.password = prompt_wifi(state.ssid, state.password)
            except ValueError as exc:
                state.last_result = RunResult(ResultCode.INPUT_ERROR, str(exc))
                print_final(state.last_result)
                continue

        if action in {"provision", "one_shot"}:
            if state.selected_device is None:
                state.last_result = RunResult(ResultCode.NOT_FOUND, "请先扫描并选择设备")
                print_final(state.last_result)
                continue
            if not state.ssid:
                state.last_result = RunResult(ResultCode.INPUT_ERROR, "请先设置 Wi-Fi SSID")
                print_final(state.last_result)
                continue

            state.last_result = asyncio.run(
                provision_device(
                    state.selected_device,
                    state.ssid,
                    state.password or "",
                    state.wait_timeout,
                    state.verbose,
                )
            )
            print_final(state.last_result)
            continue

        if action in {"device_help", "device_status", "device_whoami", "device_ifconfig", "device_shutdown"}:
            if state.selected_device is None:
                state.last_result = RunResult(ResultCode.NOT_FOUND, "请先扫描并选择设备")
                print_final(state.last_result)
                continue

            cmd = {
                "device_help": CMD_HELP,
                "device_status": CMD_STATUS,
                "device_whoami": CMD_SYS_WHOAMI,
                "device_ifconfig": CMD_NET_IFCONFIG,
                "device_shutdown": CMD_SHUTDOWN,
            }[action]
            state.last_result = asyncio.run(
                run_command(
                    state.selected_device,
                    command=cmd,
                    args={},
                    wait_timeout=state.wait_timeout,
                )
            )
            print_final(state.last_result)


def main() -> int:
    args = parse_args()
    return run_interactive(args)
