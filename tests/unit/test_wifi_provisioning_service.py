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


if __name__ == "__main__":
    unittest.main()
