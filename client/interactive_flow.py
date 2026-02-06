import argparse
import asyncio
import json
import os
import string
from pathlib import Path

from protocol.command_ids import (
    CMD_HELP,
    CMD_NET_IFCONFIG,
    CMD_PING,
    CMD_SHUTDOWN,
    CMD_STATUS,
    CMD_SYS_WHOAMI,
    CMD_WIFI_SCAN,
)
from config.defaults import (
    DEFAULT_CONNECT_RETRIES,
    DEFAULT_DEVICE_NAME,
    DEFAULT_SCAN_TIMEOUT,
    DEFAULT_WAIT_TIMEOUT,
    PASSWORD_KEY,
    SSID_KEY,
)
from client.command_client import (
    close_device_session,
    discover_devices_with_progress,
    open_device_session,
    provision_device,
    run_command,
)
from client.models import ResultCode, RunResult, SessionState
from client.prompting import ask_list, ask_secret, ask_text
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
    client = state.ble_client
    if client is None:
        return False
    try:
        return bool(client.is_connected)
    except Exception:
        return False


def _menu_title(state: SessionState) -> str:
    if _is_session_connected(state):
        lamp = "🟢"
        status = "已连接"
    else:
        lamp = "🔴"
        status = "未连接"
    return f"{lamp} {status} | 请选择操作"


def _wifi_cache_path() -> Path:
    cache_home_raw = os.environ.get("XDG_CACHE_HOME")
    cache_home = Path(cache_home_raw) if cache_home_raw else (Path.home() / ".cache")
    return cache_home / "ble-command-gateway" / "wifi_credentials.json"


def load_cached_wifi_credentials() -> tuple[str | None, str | None]:
    path = _wifi_cache_path()
    if not path.exists():
        return None, None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None, None
    if not isinstance(payload, dict):
        return None, None
    ssid_raw = payload.get(SSID_KEY)
    password_raw = payload.get(PASSWORD_KEY)
    ssid = ssid_raw.strip() if isinstance(ssid_raw, str) else None
    password = password_raw if isinstance(password_raw, str) else None
    if not ssid:
        return None, None
    return ssid, password


def save_wifi_credentials(ssid: str, password: str) -> bool:
    if not ssid.strip():
        return False
    path = _wifi_cache_path()
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        payload = {SSID_KEY: ssid, PASSWORD_KEY: password}
        path.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")
        try:
            os.chmod(path, 0o600)
        except Exception:
            pass
        return True
    except Exception:
        return False


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
    device_ready = state.selected_device is not None
    if not device_ready:
        return ask_list(
            _menu_title(state),
            choices=[
                {"value": "scan", "name": "扫描并选择设备"},
                {"value": "set_target", "name": "修改设备名过滤条件"},
                {"value": "one_shot", "name": "一键流程（扫描 -> 输入 -> 配网）"},
                {"value": "exit", "name": "退出"},
            ],
            default="scan",
        )

    return ask_list(
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
    value = ask_text("设备名过滤（包含匹配）", default=current).strip()
    return value or current


def prompt_wifi(current_ssid: str | None, current_password: str | None) -> tuple[str, str]:
    ssid = ask_text("Wi-Fi SSID", default=current_ssid or "").strip()
    if not ssid:
        raise ValueError("SSID 不能为空")

    password = ask_secret("Wi-Fi 密码（可留空）", default=current_password or "")
    if not _is_valid_wifi_password(password):
        raise ValueError("Wi-Fi 密码不合法：留空(开放网络)，或 8-63 位字符，或 64 位十六进制。")
    return ssid, password


def _is_valid_wifi_password(password: str) -> bool:
    if password == "":
        return True
    if 8 <= len(password) <= 63:
        return True
    if len(password) == 64 and all(ch in string.hexdigits for ch in password):
        return True
    return False


def scan_with_feedback(loop: asyncio.AbstractEventLoop, state: SessionState) -> tuple[list[object], list[object], int]:
    last_seen: dict[str, str] = {"text": ""}

    def _on_progress(elapsed: float, total: float, total_devices: int, matched_devices: int) -> None:
        print_scan_progress(elapsed, total, total_devices, matched_devices, last_seen["text"])

    def _on_detect(device: object) -> None:
        name = getattr(device, "name", None) or "<NoName>"
        addr = getattr(device, "address", None) or "<?>"
        last_seen["text"] = f"{name} | {addr}"

    try:
        return loop.run_until_complete(
            discover_devices_with_progress(
                target_name=state.target_name,
                timeout=state.scan_timeout,
                on_progress=_on_progress,
                on_detect=_on_detect,
            )
        )
    finally:
        end_scan_progress()


def select_device_interactive(
    all_devices: list[object],
    matched_devices: list[object],
    total_count: int,
    target_name: str,
) -> object | None:
    if len(matched_devices) == 1:
        device = matched_devices[0]
        name = getattr(device, "adv_name", None) or getattr(device, "name", None) or "<NoName>"
        addr = getattr(device, "address", None) or "<?>"
        print_note(f"已匹配唯一设备，自动选择: {name} | {addr}")
        return device

    if not matched_devices:
        print_scan_no_match(total_count, target_name)
        print_named_devices(all_devices)
        named_devices = [
            device
            for device in all_devices
            if (getattr(device, "adv_name", None) or getattr(device, "name", None))
        ]
        if not named_devices:
            return None
        if len(named_devices) == 1:
            device = named_devices[0]
            name = getattr(device, "adv_name", None) or getattr(device, "name", None) or "<NoName>"
            addr = getattr(device, "address", None) or "<?>"
            print_note(f"仅发现一个命名设备，自动选择: {name} | {addr}")
            return device
        choices = build_device_choices(named_devices)
        return ask_list("选择目标设备（无匹配时可从命名设备中选择）", choices=choices)

    choices = build_device_choices(matched_devices)
    return ask_list("选择目标设备", choices=choices)


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
    reporter("[bold cyan]BLE 配网交互模式已启动[/bold cyan]（InquirerPy）")
    if state.ssid:
        print_note(f"已加载缓存 Wi-Fi 凭据: SSID={state.ssid}")

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        def _drop_current_session() -> None:
            if state.ble_client is None:
                return
            loop.run_until_complete(close_device_session(state.ble_client))
            state.ble_client = None

        while True:
            action = choose_action(state)

            if action == "exit":
                return int(state.last_result.code) if state.last_result else 0

            if action == "set_target":
                state.target_name = prompt_target_name(state.target_name)
                continue

            if action == "set_wifi":
                try:
                    state.ssid, state.password = prompt_wifi(state.ssid, state.password)
                    save_wifi_credentials(state.ssid, state.password)
                except ValueError as exc:
                    state.last_result = RunResult(ResultCode.INPUT_ERROR, str(exc))
                    print_final(state.last_result)
                except Exception as exc:  # noqa: BLE001
                    state.last_result = RunResult(ResultCode.FAILED, f"凭据输入失败: {type(exc).__name__}: {exc}")
                    print_final(state.last_result)
                continue

            if action == "scan":
                _drop_current_session()
                all_devices, matched_devices, total_count = scan_with_feedback(loop, state)
                state.selected_device = select_device_interactive(
                    all_devices,
                    matched_devices,
                    total_count,
                    state.target_name,
                )
                if state.selected_device is not None:
                    result, client = loop.run_until_complete(
                        open_device_session(
                            state.selected_device,
                            timeout=10,
                            retries=DEFAULT_CONNECT_RETRIES,
                            reporter=reporter,
                        )
                    )
                    print_final(result)
                    if result.code is ResultCode.SUCCESS and client is not None:
                        state.ble_client = client
                    else:
                        state.selected_device = None
                        loop.run_until_complete(close_device_session(client))
                continue

            if action == "show":
                show_state(state)
                continue

            if action == "one_shot":
                _drop_current_session()
                all_devices, matched_devices, total_count = scan_with_feedback(loop, state)
                state.selected_device = select_device_interactive(
                    all_devices,
                    matched_devices,
                    total_count,
                    state.target_name,
                )
                if state.selected_device is None:
                    state.last_result = RunResult(ResultCode.NOT_FOUND, "未发现匹配设备")
                    print_final(state.last_result)
                    continue
                result, client = loop.run_until_complete(
                    open_device_session(
                        state.selected_device,
                        timeout=10,
                        retries=DEFAULT_CONNECT_RETRIES,
                        reporter=reporter,
                    )
                )
                print_final(result)
                if result.code is not ResultCode.SUCCESS:
                    state.selected_device = None
                    loop.run_until_complete(close_device_session(client))
                    continue
                if client is None:
                    state.last_result = RunResult(ResultCode.FAILED, "连接结果异常：会话未建立")
                    print_final(state.last_result)
                    state.selected_device = None
                    continue
                state.ble_client = client
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

            if action in {"provision", "one_shot"}:
                if state.selected_device is None:
                    state.last_result = RunResult(ResultCode.NOT_FOUND, "请先扫描并选择设备")
                    print_final(state.last_result)
                    continue
                if not state.ssid:
                    state.last_result = RunResult(ResultCode.INPUT_ERROR, "请先设置 Wi-Fi SSID")
                    print_final(state.last_result)
                    continue

                state.last_result = loop.run_until_complete(
                    provision_device(
                        state.selected_device,
                        state.ssid,
                        state.password or "",
                        state.wait_timeout,
                        state.verbose,
                        reporter=reporter,
                        client=state.ble_client,
                    )
                )
                print_final(state.last_result)
                if state.ble_client is not None and not state.ble_client.is_connected:
                    state.ble_client = None
                continue

            if action in DEVICE_COMMAND_MAP:
                if state.selected_device is None:
                    state.last_result = RunResult(ResultCode.NOT_FOUND, "请先扫描并选择设备")
                    print_final(state.last_result)
                    continue

                cmd = DEVICE_COMMAND_MAP[action]
                state.last_result = loop.run_until_complete(
                    run_command(
                        state.selected_device,
                        command=cmd,
                        args={},
                        wait_timeout=state.wait_timeout,
                        reporter=reporter,
                        client=state.ble_client,
                    )
                )
                if cmd == CMD_WIFI_SCAN and state.last_result.code is ResultCode.SUCCESS:
                    if not print_wifi_scan_result(state.last_result.message):
                        print_final(state.last_result)
                else:
                    print_final(state.last_result)
                if state.ble_client is not None and not state.ble_client.is_connected:
                    state.ble_client = None
    finally:
        if state.ble_client is not None:
            loop.run_until_complete(close_device_session(state.ble_client))
            state.ble_client = None
        loop.close()


def main() -> int:
    args = parse_args()
    return run_interactive(args)
