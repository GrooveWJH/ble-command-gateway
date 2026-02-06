from __future__ import annotations

import unittest

from commands.builtins import status_cmd


class StatusFormattingTests(unittest.TestCase):
    def test_connected_wifi_includes_ip(self) -> None:
        text = '{"state":"connected","ssid":"Yundrone_MOffice","ip":"192.168.10.198"}'
        rendered = status_cmd._format_wifi(True, text)
        self.assertEqual(rendered, "connected (SSID=Yundrone_MOffice, IP=192.168.10.198)")

    def test_connected_wifi_without_ip_shows_unknown(self) -> None:
        text = '{"state":"connected","ssid":"Yundrone_MOffice","ip":""}'
        rendered = status_cmd._format_wifi(True, text)
        self.assertEqual(rendered, "connected (SSID=Yundrone_MOffice, IP=unknown)")


if __name__ == "__main__":
    unittest.main()
