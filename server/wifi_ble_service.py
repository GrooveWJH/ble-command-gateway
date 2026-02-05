#!/usr/bin/env python3
import argparse
import asyncio
import ipaddress
import json
import logging
import shlex
import socket
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT_DIR = Path(__file__).resolve().parents[1]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from config import (  # noqa: E402
    CHAR_READ_UUID,
    CHAR_WRITE_UUID,
    DEFAULT_CONNECT_TIMEOUT,
    DEFAULT_DEVICE_NAME,
    PASSWORD_KEY,
    SERVICE_UUID,
    SSID_KEY,
    STATUS_BUSY_PREFIX,
    STATUS_CONNECTING,
    STATUS_FAIL_PREFIX,
    STATUS_STANDBY,
    STATUS_SUCCESS_PREFIX,
)

from server.ble_runtime import load_bless_symbols, stop_bless_server, write_properties  # noqa: E402
from server.preflight import format_preflight_report, run_preflight_checks  # noqa: E402


class BLEProvisioningServer:
    def __init__(self, device_name: str, interface: str | None, connect_timeout: int, adapter: str | None) -> None:
        self.device_name = device_name
        self.interface = interface
        self.connect_timeout = connect_timeout
        self.server: Any | None = None
        self.adapter = adapter
        self.logger = logging.getLogger(device_name)
        self._connect_lock = asyncio.Lock()

    async def start(self) -> None:
        try:
            bless_server_cls, gatt_props, gatt_perms = load_bless_symbols()
        except Exception as exc:
            raise RuntimeError("Failed to load bless runtime symbols. Check bless/bleak compatibility.") from exc

        server = bless_server_cls(name=self.device_name, loop=asyncio.get_running_loop(), adapter=self.adapter)

        await server.add_new_service(SERVICE_UUID)
        await server.add_new_characteristic(
            SERVICE_UUID,
            CHAR_WRITE_UUID,
            properties=write_properties(gatt_props),
            permissions=gatt_perms.writeable,
            value=bytearray(),
        )
        await server.add_new_characteristic(
            SERVICE_UUID,
            CHAR_READ_UUID,
            properties=gatt_props.read | gatt_props.notify,
            permissions=gatt_perms.readable,
            value=bytearray(STATUS_STANDBY, "utf-8"),
        )

        write_char = server.get_characteristic(CHAR_WRITE_UUID)
        if write_char is None:
            raise RuntimeError("Write characteristic not found after setup")
        setattr(server, "write_request_func", self._handle_write_request)

        try:
            await server.start()
        except Exception as exc:  # noqa: BLE001
            adapter_name = self.adapter or "<auto>"
            raise RuntimeError(
                "Failed to register BLE advertisement on "
                f"adapter {adapter_name}: {exc}. "
                "Try: sudo, `hciconfig hci0 up`, and `systemctl restart bluetooth`."
            ) from exc

        self.server = server
        self.logger.info("BLE service started as %s", self.device_name)

        while True:
            await asyncio.sleep(1)

    async def stop(self) -> None:
        if self.server is None:
            return

        server = self.server
        self.server = None
        self.logger.info("Stopping BLE advertisement")

        stop_fn = getattr(server, "stop", None)
        if not callable(stop_fn):
            self.logger.warning("BlessServer.stop is unavailable; skip shutdown")
            return

        try:
            await stop_bless_server(server, timeout=5)
            self.logger.info("BLE advertisement stopped")
        except Exception as exc:  # noqa: BLE001
            self.logger.warning("BLE shutdown failed: %s", exc)

    def _update_status(self, message: str) -> None:
        if self.server is None:
            return

        char = self.server.get_characteristic(CHAR_READ_UUID)
        if char is None:
            self.logger.error("Read characteristic missing, unable to publish status")
            return

        char.value = bytearray(message, "utf-8")
        try:
            self.server.update_value(SERVICE_UUID, CHAR_READ_UUID)
        except Exception:
            pass
        self.logger.info("Status -> %s", message)

    def _handle_write_request(self, characteristic: Any, value: Any, **kwargs: Any) -> None:
        del characteristic, kwargs
        try:
            raw = bytes(value).decode("utf-8")
            req = json.loads(raw)
            ssid = str(req.get(SSID_KEY, "")).strip()
            password = str(req.get(PASSWORD_KEY, ""))
            if not ssid:
                self._update_status(f"{STATUS_FAIL_PREFIX}Missing SSID")
                return
            asyncio.create_task(self._provision_wifi(ssid, password))
        except json.JSONDecodeError:
            self._update_status(f"{STATUS_FAIL_PREFIX}Invalid JSON")
        except Exception as exc:
            self.logger.exception("Failed to parse write request")
            self._update_status(f"{STATUS_FAIL_PREFIX}{type(exc).__name__}")

    async def _provision_wifi(self, ssid: str, password: str) -> None:
        if self._connect_lock.locked():
            self._update_status(f"{STATUS_BUSY_PREFIX}Provisioning in progress")
            return

        async with self._connect_lock:
            self._update_status(STATUS_CONNECTING)
            ok, message = await self._connect_wifi_with_nmcli(ssid, password)
            if not ok:
                self._update_status(f"{STATUS_FAIL_PREFIX}{message}")
                return

            ip = await self._wait_for_ip(timeout=15)
            if ip:
                self._update_status(f"{STATUS_SUCCESS_PREFIX}{ip}")
            else:
                self._update_status(f"{STATUS_FAIL_PREFIX}No IP assigned")

    async def _connect_wifi_with_nmcli(self, ssid: str, password: str) -> tuple[bool, str]:
        cmd = ["nmcli", "device", "wifi", "connect", ssid]
        if password:
            cmd += ["password", password]
        if self.interface:
            cmd += ["ifname", self.interface]

        self.logger.info("Executing: %s", " ".join(shlex.quote(part) for part in cmd))
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=self.connect_timeout)
        except asyncio.TimeoutError:
            proc.kill()
            await proc.communicate()
            return False, "nmcli timeout"

        if proc.returncode == 0:
            return True, "Connected"

        err = stderr.decode("utf-8", errors="ignore").strip()
        out = stdout.decode("utf-8", errors="ignore").strip()
        text = err or out or f"nmcli rc={proc.returncode}"
        return False, text[:160]

    async def _wait_for_ip(self, timeout: int) -> str | None:
        deadline = asyncio.get_running_loop().time() + timeout
        while asyncio.get_running_loop().time() < deadline:
            ip = self._get_primary_ipv4()
            if ip:
                return ip
            await asyncio.sleep(1)
        return None

    def _get_primary_ipv4(self) -> str | None:
        if self.interface:
            ip = self._get_ipv4_for_interface(self.interface)
            if ip:
                return ip

        try:
            output = subprocess.check_output(["hostname", "-I"], text=True).strip()
        except Exception:
            output = ""

        for token in output.split():
            try:
                ip_obj = ipaddress.ip_address(token)
                if isinstance(ip_obj, ipaddress.IPv4Address):
                    return str(ip_obj)
            except ValueError:
                continue

        try:
            host_ip = socket.gethostbyname(socket.gethostname())
            ip_obj = ipaddress.ip_address(host_ip)
            if isinstance(ip_obj, ipaddress.IPv4Address) and not ip_obj.is_loopback:
                return str(ip_obj)
        except Exception:
            pass

        return None

    @staticmethod
    def _get_ipv4_for_interface(interface: str) -> str | None:
        try:
            out = subprocess.check_output(["ip", "-4", "addr", "show", interface], text=True)
        except Exception:
            return None

        for line in out.splitlines():
            line = line.strip()
            if line.startswith("inet "):
                token = line.split()[1]
                return token.split("/")[0]
        return None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="BLE-based Wi-Fi provisioning server")
    parser.add_argument("--device-name", default=DEFAULT_DEVICE_NAME, help="BLE advertised name")
    parser.add_argument("--ifname", default=None, help="Wi-Fi interface name, e.g. wlan0")
    parser.add_argument("--adapter", default="hci0", help="Bluetooth adapter, e.g. hci0 (empty for auto)")
    parser.add_argument("--connect-timeout", type=int, default=DEFAULT_CONNECT_TIMEOUT, help="nmcli connect timeout seconds")
    parser.add_argument("--log-level", default="INFO", choices=["DEBUG", "INFO", "WARNING", "ERROR"])
    return parser.parse_args()


async def main() -> None:
    args = parse_args()
    logging.basicConfig(
        level=getattr(logging, args.log_level),
        format="%(asctime)s %(levelname)s %(name)s | %(message)s",
    )

    adapter = args.adapter.strip() or "hci0"
    preflight = run_preflight_checks(adapter)
    print(format_preflight_report(preflight))
    if not preflight.ok:
        raise SystemExit("Preflight failed. Resolve the checks above before starting BLE advertising.")

    server = BLEProvisioningServer(
        device_name=args.device_name,
        interface=args.ifname,
        connect_timeout=args.connect_timeout,
        adapter=adapter,
    )
    try:
        await server.start()
    finally:
        await server.stop()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n[server] interrupted")
