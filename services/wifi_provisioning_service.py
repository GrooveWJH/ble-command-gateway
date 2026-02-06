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
        self.logger.info(
            "[PROVISION] connect_and_get_ip begin ssid=%s ifname=%s connect_timeout=%ss ip_timeout=%ss",
            ssid,
            self.interface or "<auto>",
            self.connect_timeout,
            ip_timeout,
        )
        current_ssid = self._get_connected_ssid()
        self.logger.debug("[PROVISION] current_ssid=%s target_ssid=%s", current_ssid, ssid)
        if current_ssid == ssid:
            self.logger.info("Wi-Fi already connected to requested SSID=%s; skipping nmcli connect", ssid)
            ip = await self._wait_for_ip(timeout=ip_timeout)
            if ip is None:
                return False, f"Already connected to {ssid}, but no IP assigned", None
            self.logger.info("[PROVISION] existing-link ip-ready ssid=%s ip=%s", ssid, ip)
            return True, "Already connected", ip

        ok, message = await self._connect_wifi_with_nmcli(ssid, password)
        if not ok:
            self.logger.info("[PROVISION] nmcli connect failed ssid=%s reason=%s", ssid, message)
            return False, message, None

        ip = await self._wait_for_ip(timeout=ip_timeout)
        if ip is None:
            self.logger.info("[PROVISION] connected but no ip ssid=%s", ssid)
            return False, "No IP assigned", None
        self.logger.info("[PROVISION] connected+ip ssid=%s ip=%s", ssid, ip)
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
        started = asyncio.get_running_loop().time()

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
            elapsed = asyncio.get_running_loop().time() - started
            self.logger.info("[PROVISION] nmcli timeout ssid=%s elapsed=%.1fs", ssid, elapsed)
            return False, "nmcli timeout"

        elapsed = asyncio.get_running_loop().time() - started
        out_text = stdout.decode("utf-8", errors="ignore").strip()
        err_text = stderr.decode("utf-8", errors="ignore").strip()
        self.logger.debug("[PROVISION] nmcli stdout=%s", _preview(out_text, 220))
        self.logger.debug("[PROVISION] nmcli stderr=%s", _preview(err_text, 220))
        if proc.returncode == 0:
            self.logger.info("[PROVISION] nmcli success ssid=%s elapsed=%.1fs", ssid, elapsed)
            return True, "Connected"

        text = err_text or out_text or f"nmcli rc={proc.returncode}"
        self.logger.info(
            "[PROVISION] nmcli failed ssid=%s rc=%s elapsed=%.1fs text=%s",
            ssid,
            proc.returncode,
            elapsed,
            _preview(text, 160),
        )
        return False, text[:160]

    async def _wait_for_ip(self, timeout: int) -> str | None:
        self.logger.info("[PROVISION] wait ip start timeout=%ss ifname=%s", timeout, self.interface or "<auto>")
        deadline = asyncio.get_running_loop().time() + timeout
        poll = 0
        while asyncio.get_running_loop().time() < deadline:
            poll += 1
            ip = self._get_primary_ipv4()
            if ip:
                self.logger.info("[PROVISION] wait ip success poll=%s ip=%s", poll, ip)
                return ip
            if poll == 1 or poll % 3 == 0:
                elapsed = int(timeout - max(deadline - asyncio.get_running_loop().time(), 0))
                self.logger.info("[PROVISION] wait ip poll=%s elapsed=%ss no-ip-yet", poll, elapsed)
            else:
                self.logger.debug("[PROVISION] wait ip poll=%s no-ip-yet", poll)
            await asyncio.sleep(1)
        self.logger.info("[PROVISION] wait ip timeout after %ss", timeout)
        return None

    def _get_connected_ssid(self) -> str | None:
        cmd = ["nmcli", "-t", "-f", "ACTIVE,SSID", "device", "wifi", "list"]
        if self.interface:
            cmd += ["ifname", self.interface]

        try:
            output = subprocess.check_output(cmd, text=True).strip()
        except Exception:
            self.logger.debug("[PROVISION] get_connected_ssid failed", exc_info=True)
            return None

        for raw_line in output.splitlines():
            line = raw_line.strip()
            if not line:
                continue
            active, sep, ssid = line.partition(":")
            if not sep:
                continue
            if active.strip().lower() not in {"yes", "*"}:
                continue
            cleaned_ssid = ssid.strip().replace("\\:", ":")
            if cleaned_ssid:
                return cleaned_ssid
        return None

    def _get_primary_ipv4(self) -> str | None:
        if self.interface:
            ip = self._get_ipv4_for_interface(self.interface)
            if ip:
                self.logger.debug("[PROVISION] ip from interface=%s value=%s", self.interface, ip)
                return ip

        try:
            output = subprocess.check_output(["hostname", "-I"], text=True).strip()
        except Exception:
            output = ""

        for token in output.split():
            try:
                ip_obj = ipaddress.ip_address(token)
                if isinstance(ip_obj, ipaddress.IPv4Address):
                    self.logger.debug("[PROVISION] ip from hostname -I value=%s", ip_obj)
                    return str(ip_obj)
            except ValueError:
                continue

        try:
            host_ip = socket.gethostbyname(socket.gethostname())
            ip_obj = ipaddress.ip_address(host_ip)
            if isinstance(ip_obj, ipaddress.IPv4Address) and not ip_obj.is_loopback:
                self.logger.debug("[PROVISION] ip from gethostbyname value=%s", ip_obj)
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


def _preview(text: str, limit: int = 180) -> str:
    compact = text.replace("\n", "\\n")
    if len(compact) <= limit:
        return compact
    return f"{compact[:limit]}...(len={len(compact)})"
