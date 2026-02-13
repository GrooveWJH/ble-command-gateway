from __future__ import annotations

import logging
import unittest
from unittest.mock import AsyncMock, patch

from services.wifi_provisioning_service import WifiProvisioningService


class WifiProvisioningServiceTests(unittest.IsolatedAsyncioTestCase):
    async def test_same_ssid_short_circuits_connect(self) -> None:
        service = WifiProvisioningService(interface="wlan0", connect_timeout=30, logger=logging.getLogger("test"))

        with (
            patch.object(service, "_get_connected_ssid", return_value="OfficeWiFi"),
            patch.object(service, "_wait_for_ip", new=AsyncMock(return_value="192.168.1.20")),
            patch.object(service, "_connect_wifi_with_nmcli", new=AsyncMock()) as connect_mock,
        ):
            ok, message, ip = await service.connect_and_get_ip("OfficeWiFi", "secret", ip_timeout=15)

        self.assertTrue(ok)
        self.assertEqual(message, "Already connected")
        self.assertEqual(ip, "192.168.1.20")
        connect_mock.assert_not_called()

    async def test_different_ssid_still_runs_connect(self) -> None:
        service = WifiProvisioningService(interface="wlan0", connect_timeout=30, logger=logging.getLogger("test"))

        with (
            patch.object(service, "_get_connected_ssid", return_value="OfficeWiFi"),
            patch.object(service, "_connect_wifi_with_nmcli", new=AsyncMock(return_value=(True, "Connected"))) as connect_mock,
            patch.object(service, "_wait_for_ip", new=AsyncMock(return_value="192.168.1.30")),
        ):
            ok, message, ip = await service.connect_and_get_ip("LabWiFi", "secret", ip_timeout=15)

        self.assertTrue(ok)
        self.assertEqual(message, "Connected")
        self.assertEqual(ip, "192.168.1.30")
        connect_mock.assert_awaited_once()

    def test_primary_ip_prefers_connected_wifi_interface_when_ifname_missing(self) -> None:
        service = WifiProvisioningService(interface=None, connect_timeout=30, logger=logging.getLogger("test"))

        with (
            patch.object(service, "_get_connected_wifi_interface", return_value="wlan0"),
            patch.object(service, "_get_ipv4_for_interface", side_effect=lambda ifname: "198.51.100.228" if ifname == "wlan0" else None),
            patch("subprocess.check_output", return_value="203.0.113.5 172.17.0.1"),
        ):
            ip = service._get_primary_ipv4()

        self.assertEqual(ip, "198.51.100.228")

    def test_primary_ip_falls_back_to_hostname_i_when_no_wifi_interface(self) -> None:
        service = WifiProvisioningService(interface=None, connect_timeout=30, logger=logging.getLogger("test"))

        with (
            patch.object(service, "_get_connected_wifi_interface", return_value=None),
            patch.object(service, "_get_ipv4_for_interface", return_value=None),
            patch("subprocess.check_output", return_value="203.0.113.5 172.17.0.1"),
        ):
            ip = service._get_primary_ipv4()

        self.assertEqual(ip, "203.0.113.5")


if __name__ == "__main__":
    unittest.main()
