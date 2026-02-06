import argparse
import asyncio
import getpass
import logging
import socket
import sys
import time
from pathlib import Path
import subprocess
from typing import Any

from bless.backends.attribute import GATTAttributePermissions
from bless.backends.characteristic import GATTCharacteristicProperties
ROOT_DIR = Path(__file__).resolve().parents[2]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from server.preflight import detect_default_adapter  # noqa: E402

def _make_server(*_args: Any, **_kwargs: Any) -> Any:
    try:
        if sys.platform == "darwin":
            from bless.backends.corebluetooth.server import BlessServerCoreBluetooth as _BlessServer
        elif sys.platform.startswith("linux"):
            from bless.backends.bluezdbus.server import BlessServerBlueZDBus as _BlessServer
        elif sys.platform.startswith("win"):
            from bless.backends.winrt.server import BlessServerWinRT as _BlessServer
        else:
            raise RuntimeError(f"Unsupported platform: {sys.platform}")
    except Exception as exc:
        raise RuntimeError(
            "BlessServer backend unavailable. Ensure bless and platform deps are installed."
        ) from exc
    return _BlessServer(*_args, **_kwargs)


# Nordic UART UUIDs
SERVICE_UUID = "6E400001-B5A3-F393-E0A9-E50E24DCCA9E"
RX_UUID      = "6E400002-B5A3-F393-E0A9-E50E24DCCA9E" # iPhone Write
TX_UUID      = "6E400003-B5A3-F393-E0A9-E50E24DCCA9E" # iPhone Read/Notify

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("MacUART")
_last_activity: float | None = None
_last_no_subscriber_log: float | None = None
_shutdown_requested = False
_last_reply: str = "Ready"
_server_ref: Any | None = None
_tx_char_ref: Any | None = None

def on_write(characteristic: Any, value: Any, **kwargs: Any):
    """
    当 iPhone 发消息过来时，会触发这里
    """
    global _last_activity
    _last_activity = time.monotonic()
    try:
        if isinstance(value, memoryview):
            value = value.tobytes()
        try:
            data = bytes(value)
        except Exception:
            data = b""
        if data:
            text = data.decode("utf-8", errors="replace")
            print(f"\n📩 [收到 iPhone]: {text}")
            _handle_command(text.strip())
            return
        print(f"\n📩 [收到 未知类型]: {value}")
    except Exception as e:
        print(f"\n📩 [解码失败]: {value} ({e})")


def on_read(_characteristic: Any, **_kwargs: Any) -> bytearray:
    global _last_activity
    _last_activity = time.monotonic()
    return bytearray(_last_reply.encode("utf-8"))


def _send_reply(text: str) -> None:
    global _last_reply
    _last_reply = text
    if _server_ref is None or _tx_char_ref is None:
        return
    _tx_char_ref.value = bytearray(text.encode("utf-8"))
    _server_ref.update_value(SERVICE_UUID, TX_UUID)


def _handle_command(cmd: str) -> None:
    global _shutdown_requested
    if cmd == "cmd-username":
        _send_reply(getpass.getuser())
        return
    if cmd == "cmd-hostname":
        _send_reply(socket.gethostname())
        return
    if cmd == "shutdown":
        _send_reply("OK")
        _shutdown_requested = True
        return
    if cmd:
        _send_reply("ERR: unknown command")


def _is_connected(server: Any, tx_char: Any) -> tuple[bool, bool]:
    subscribers = getattr(tx_char, "subscribed_list", None)
    if subscribers is not None:
        return bool(subscribers), True
    manager = getattr(server, "peripheral_manager", None)
    if manager is not None:
        if getattr(manager, "isAdvertising", None) is False:
            return True, False
    if _last_activity is not None:
        return (time.monotonic() - _last_activity) < 2.0, False
    return False, False

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="BLE UART demo (macOS, bless)")
    parser.add_argument("--device-name", default="Mac_UART_Demo", help="advertised device name")
    parser.add_argument("--adapter", default="auto", help="Bluetooth adapter (auto, hci0, hci1)")
    return parser.parse_args()


def _set_pairable_off(adapter: str) -> None:
    state = "off"
    try:
        subprocess.run(["btmgmt", "-i", adapter, "pairable", state], check=False, capture_output=True, text=True)
        subprocess.run(["btmgmt", "-i", adapter, "bondable", state], check=False, capture_output=True, text=True)
        subprocess.run(["btmgmt", "-i", adapter, "discoverable", state], check=False, capture_output=True, text=True)
        logger.info("Adapter %s pairable=%s bondable=%s discoverable=%s", adapter, state, state, state)
    except Exception:
        pass

    try:
        subprocess.run(
            ["bluetoothctl"],
            input=f"pairable {state}\nbondable {state}\ndiscoverable {state}\nexit\n",
            text=True,
            check=False,
            capture_output=True,
        )
        logger.info("Adapter pairable=%s bondable=%s discoverable=%s (bluetoothctl)", state, state, state)
    except Exception as exc:
        logger.warning("Failed to set pairable state: %s", exc)

    try:
        proc = subprocess.run(["btmgmt", "-i", adapter, "info"], check=False, capture_output=True, text=True)
        if proc.stdout.strip():
            for line in proc.stdout.splitlines():
                if "current settings" in line:
                    logger.info("Adapter settings: %s", line.strip())
    except Exception:
        pass


async def run(loop: asyncio.AbstractEventLoop, device_name: str, adapter: str | None):
    # 1. 启动 Server
    kwargs = {"name": device_name, "loop": loop}
    if adapter:
        kwargs["adapter"] = adapter
    server = _make_server(**kwargs)
    
    logger.info("正在启动蓝牙服务...")
    
    # 2. 添加 UART 服务
    await server.add_new_service(SERVICE_UUID)
    
    # 3. 添加 RX 特征 (允许 iPhone 写入)
    await server.add_new_characteristic(
        SERVICE_UUID,
        RX_UUID,
        properties=(
            GATTCharacteristicProperties.write
            | GATTCharacteristicProperties.write_without_response
        ),
        permissions=GATTAttributePermissions.writeable,
        value=None,
    )
    
    # 4. 添加 TX 特征 (允许 iPhone 订阅通知)
    await server.add_new_characteristic(
        SERVICE_UUID,
        TX_UUID,
        properties=(
            GATTCharacteristicProperties.notify
            | GATTCharacteristicProperties.read
        ),
        permissions=GATTAttributePermissions.readable,
        value=None,
    )

    # 5. 绑定回调（CoreBluetooth backend expects it on server)
    setattr(server, "write_request_func", on_write)
    setattr(server, "read_request_func", on_read)

    # 6. 开始广播
    await server.start()
    logger.info("✅ 服务已启动！")
    logger.info("Advertising as: %s", device_name)
    logger.info("Service UUID: %s", SERVICE_UUID)
    logger.info("RX UUID (write): %s", RX_UUID)
    logger.info("TX UUID (notify/read): %s", TX_UUID)
    
    global _server_ref, _tx_char_ref
    _server_ref = server
    _tx_char_ref = server.get_characteristic(TX_UUID)

    logger.info("🚀 命令模式已启动 (cmd-username / cmd-hostname / shutdown)")
    while not _shutdown_requested:
        await asyncio.sleep(0.2)

    logger.info("🛑 shutdown requested, stopping server...")
    await server.stop()

if __name__ == "__main__":
    args = parse_args()
    adapter: str | None = None
    if sys.platform.startswith("linux"):
        linux_adapter = args.adapter.strip()
        if linux_adapter in {"", "auto"}:
            linux_adapter = detect_default_adapter()
            if linux_adapter is None:
                raise SystemExit("No bluetooth adapter detected (no hci* found)")
        adapter = linux_adapter
        _set_pairable_off(adapter)
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        loop.run_until_complete(run(loop, args.device_name, adapter))
    except KeyboardInterrupt:
        logger.info("正在停止...")
    finally:
        loop.close()
