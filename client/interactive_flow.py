from __future__ import annotations

import argparse
import asyncio
import string

from protocol.command_ids import (
    CMD_HELP,
    CMD_NET_IFCONFIG,
    CMD_PING,
    CMD_SHUTDOWN,
    CMD_STATUS,
    CMD_SYS_WHOAMI,
    CMD_WIFI_SCAN,
)
from config.defaults import DEFAULT_DEVICE_NAME, DEFAULT_SCAN_TIMEOUT, DEFAULT_WAIT_TIMEOUT
from client.cli_controller import CLIController
from client.credential_store import load_cached_wifi_credentials, save_wifi_credentials
from client.library_api import BleGatewayClient
from client.library_models import DeviceInfo
from client.models import ResultCode, RunResult, SessionState
from client.render import (
    build_device_choices,
    end_scan_progress,
    make_ui_reporter,
    print_note,
    print_final,
    print_named_devices,
    print_scan_no_match,
    print_scan_progress,
    print_wifi_scan_result,
    show_state,
)


def _ask_list(*args: object, **kwargs: object) -> str:
    from client.prompting import ask_list

    return ask_list(*args, **kwargs)


def _ask_text(*args: object, **kwargs: object) -> str:
    from client.prompting import ask_text

    return ask_text(*args, **kwargs)


def _ask_secret(*args: object, **kwargs: object) -> str:
    from client.prompting import ask_secret

    return ask_secret(*args, **kwargs)


DEVICE_COMMAND_ITEMS: tuple[tuple[str, str, str], ...] = (
    ("device_help", "查看设备 help", CMD_HELP),
    ("device_ping", "查看设备 ping", CMD_PING),
    ("device_status", "查看设备 status", CMD_STATUS),
    ("device_whoami", "查看设备 whoami", CMD_SYS_WHOAMI),
    ("device_ifconfig", "查看设备 ifconfig", CMD_NET_IFCONFIG),
    ("device_wifi_scan", "查看设备 Wi-Fi 扫描(5s)", CMD_WIFI_SCAN),
    ("device_shutdown", "执行设备 shutdown", CMD_SHUTDOWN),
)

DEVICE_COMMAND_MAP: dict[str, str] = {action: cmd for action, _label, cmd in DEVICE_COMMAND_ITEMS}


def _is_session_connected(state: SessionState) -> bool:
    session = state.active_session
    if session is None:
        return False
    return bool(session.is_connected)


def _menu_title(state: SessionState) -> str:
    if _is_session_connected(state):
        lamp = "🟢"
        status = "已连接"
    else:
        lamp = "🔴"
        status = "未连接"
    return f"{lamp} {status} | 请选择操作"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Interactive BLE Wi-Fi provisioning client (InquirerPy)",
    )
    parser.add_argument("--target-name", default=DEFAULT_DEVICE_NAME, help="BLE device name contains this string")
    parser.add_argument("--scan-timeout", type=int, default=DEFAULT_SCAN_TIMEOUT, help="BLE scan timeout seconds")
    parser.add_argument("--wait-timeout", type=int, default=DEFAULT_WAIT_TIMEOUT, help="Wait status timeout seconds")
    parser.add_argument("--verbose", action="store_true", help="Print all status polling logs")
    return parser.parse_args()


def choose_action(state: SessionState) -> str:
    if state.selected_device is None:
        return _ask_list(
            _menu_title(state),
            choices=[
                {"value": "scan", "name": "扫描并选择设备"},
                {"value": "set_target", "name": "修改设备名过滤条件"},
                {"value": "one_shot", "name": "一键流程（扫描 -> 输入 -> 配网）"},
                {"value": "exit", "name": "退出"},
            ],
            default="scan",
        )

    return _ask_list(
        _menu_title(state),
        choices=[
            {"value": "scan", "name": "扫描并选择设备"},
            {"value": "set_target", "name": "修改设备名过滤条件"},
            {"value": "set_wifi", "name": "设置 Wi-Fi 凭据"},
            {"value": "provision", "name": "执行配网"},
            *({"value": action, "name": label} for action, label, _cmd in DEVICE_COMMAND_ITEMS),
            {"value": "one_shot", "name": "一键流程（扫描 -> 输入 -> 配网）"},
            {"value": "show", "name": "查看当前会话状态"},
            {"value": "exit", "name": "退出"},
        ],
        default="scan",
    )


def prompt_target_name(current: str) -> str:
    value = _ask_text("设备名过滤（包含匹配）", default=current).strip()
    return value or current


def _is_valid_wifi_password(password: str) -> bool:
    if password == "":
        return True
    if 8 <= len(password) <= 63:
        return True
    if len(password) == 64 and all(ch in string.hexdigits for ch in password):
        return True
    return False


def prompt_wifi(current_ssid: str | None, current_password: str | None) -> tuple[str, str]:
    ssid = _ask_text("Wi-Fi SSID", default=current_ssid or "").strip()
    if not ssid:
        raise ValueError("SSID 不能为空")

    password = _ask_secret("Wi-Fi 密码（可留空）", default=current_password or "")
    if not _is_valid_wifi_password(password):
        raise ValueError("Wi-Fi 密码不合法：留空(开放网络)，或 8-63 位字符，或 64 位十六进制。")
    return ssid, password


def scan_with_feedback(
    loop: asyncio.AbstractEventLoop,
    gateway: BleGatewayClient,
    state: SessionState,
) -> tuple[list[DeviceInfo], list[DeviceInfo], int]:
    last_seen: dict[str, str] = {"text": ""}

    def _on_progress(elapsed: float, total: float, total_devices: int, matched_devices: int) -> None:
        print_scan_progress(elapsed, total, total_devices, matched_devices, last_seen["text"])

    def _on_detect(device: DeviceInfo) -> None:
        name = device.adv_name or device.name or "<NoName>"
        last_seen["text"] = f"{name} | {device.address}"

    try:
        snapshot = loop.run_until_complete(
            gateway.scan_snapshot(
                target_name=state.target_name,
                timeout=state.scan_timeout,
                on_progress=_on_progress,
                on_detect=_on_detect,
            )
        )
        return list(snapshot.devices), list(snapshot.matched), snapshot.total_count
    finally:
        end_scan_progress()


def select_device_interactive(
    all_devices: list[DeviceInfo],
    matched_devices: list[DeviceInfo],
    total_count: int,
    target_name: str,
) -> DeviceInfo | None:
    if len(matched_devices) == 1:
        device = matched_devices[0]
        print_note(f"已匹配唯一设备，自动选择: {device.adv_name or device.name or '<NoName>'} | {device.address}")
        return device

    if not matched_devices:
        print_scan_no_match(total_count, target_name)
        print_named_devices(all_devices)
        named_devices = [device for device in all_devices if (device.adv_name or device.name)]
        if not named_devices:
            return None
        if len(named_devices) == 1:
            device = named_devices[0]
            print_note(f"仅发现一个命名设备，自动选择: {device.adv_name or device.name or '<NoName>'} | {device.address}")
            return device
        choices = build_device_choices(named_devices)
        return _ask_list("选择目标设备（无匹配时可从命名设备中选择）", choices=choices)

    choices = build_device_choices(matched_devices)
    return _ask_list("选择目标设备", choices=choices)


def run_interactive(args: argparse.Namespace) -> int:
    cached_ssid, cached_password = load_cached_wifi_credentials()
    state = SessionState(
        target_name=args.target_name,
        scan_timeout=args.scan_timeout,
        wait_timeout=args.wait_timeout,
        verbose=args.verbose,
        ssid=cached_ssid,
        password=cached_password,
    )

    reporter = make_ui_reporter()
    gateway = BleGatewayClient(
        target_name=state.target_name,
        scan_timeout=state.scan_timeout,
        wait_timeout=state.wait_timeout,
        verbose=state.verbose,
        reporter=reporter,
    )
    controller = CLIController(state, gateway)

    reporter("[bold cyan]BLE 配网交互模式已启动[/bold cyan]（InquirerPy）")
    if state.ssid:
        print_note(f"已加载缓存 Wi-Fi 凭据: SSID={state.ssid}")

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        while True:
            action = choose_action(state)

            if action == "exit":
                return int(state.last_result.code) if state.last_result else 0

            if action == "set_target":
                state.target_name = prompt_target_name(state.target_name)
                gateway.target_name = state.target_name
                continue

            if action == "set_wifi":
                try:
                    state.ssid, state.password = prompt_wifi(state.ssid, state.password)
                    save_wifi_credentials(state.ssid, state.password)
                except ValueError as exc:
                    state.last_result = RunResult(ResultCode.INPUT_ERROR, str(exc))
                except Exception as exc:  # noqa: BLE001
                    state.last_result = RunResult(ResultCode.FAILED, f"凭据输入失败: {type(exc).__name__}: {exc}")
                print_final(state.last_result)
                continue

            if action in {"scan", "one_shot"}:
                loop.run_until_complete(controller.close_active_session())
                all_devices, matched_devices, total_count = scan_with_feedback(loop, gateway, state)
                selected = select_device_interactive(all_devices, matched_devices, total_count, state.target_name)
                state.selected_device = selected
                if selected is None:
                    state.last_result = RunResult(ResultCode.NOT_FOUND, "未发现匹配设备")
                    print_final(state.last_result)
                    continue
                state.last_result = loop.run_until_complete(controller.connect_device(selected))
                print_final(state.last_result)
                if state.last_result.code is not ResultCode.SUCCESS:
                    continue
                if action == "scan":
                    continue
                try:
                    state.ssid, state.password = prompt_wifi(state.ssid, state.password)
                    save_wifi_credentials(state.ssid, state.password)
                except ValueError as exc:
                    state.last_result = RunResult(ResultCode.INPUT_ERROR, str(exc))
                    print_final(state.last_result)
                    continue
                except Exception as exc:  # noqa: BLE001
                    state.last_result = RunResult(ResultCode.FAILED, f"凭据输入失败: {type(exc).__name__}: {exc}")
                    print_final(state.last_result)
                    continue

            if action == "show":
                show_state(state)
                continue

            if action in {"provision", "one_shot"}:
                state.last_result = loop.run_until_complete(
                    controller.provision_current(
                        ssid=state.ssid or "",
                        password=state.password or "",
                        timeout=state.wait_timeout,
                        verbose=state.verbose,
                    )
                )
                print_final(state.last_result)
                continue

            if action in DEVICE_COMMAND_MAP:
                cmd = DEVICE_COMMAND_MAP[action]
                state.last_result = loop.run_until_complete(
                    controller.run_current_command(command=cmd, timeout=state.wait_timeout)
                )
                if cmd == CMD_WIFI_SCAN and state.last_result.code is ResultCode.SUCCESS:
                    if not print_wifi_scan_result(state.last_result.message):
                        print_final(state.last_result)
                else:
                    print_final(state.last_result)
    finally:
        loop.run_until_complete(controller.close_active_session())
        loop.close()


def main() -> int:
    args = parse_args()
    return run_interactive(args)
