"""Wi-Fi provisioning operations separated from BLE runtime."""

from __future__ import annotations

import asyncio
import ipaddress
import logging
import shlex
import socket
import subprocess


class WifiProvisioningService:
    def __init__(self, interface: str | None, connect_timeout: int, logger: logging.Logger) -> None:
        self.interface = interface
        self.connect_timeout = connect_timeout
        self.logger = logger

    async def connect_and_get_ip(self, ssid: str, password: str, ip_timeout: int = 15) -> tuple[bool, str, str | None]:
        ok, message = await self._connect_wifi_with_nmcli(ssid, password)
        if not ok:
            return False, message, None

        ip = await self._wait_for_ip(timeout=ip_timeout)
        if ip is None:
            return False, "No IP assigned", None
        return True, "Connected", ip

    async def _connect_wifi_with_nmcli(self, ssid: str, password: str) -> tuple[bool, str]:
        cmd = ["nmcli", "device", "wifi", "connect", ssid]
        if password:
            cmd += ["password", password]
        if self.interface:
            cmd += ["ifname", self.interface]

        safe_cmd = ["nmcli", "device", "wifi", "connect", ssid]
        if password:
            safe_cmd += ["password", "***"]
        if self.interface:
            safe_cmd += ["ifname", self.interface]
        self.logger.info("Executing: %s", " ".join(shlex.quote(part) for part in safe_cmd))

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
