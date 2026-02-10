from __future__ import annotations

import unittest

from commands.builtins import status_cmd


class StatusFormattingTests(unittest.TestCase):
    def test_connected_wifi_omits_ip_in_wifi_field(self) -> None:
        text = '{"state":"connected","ssid":"Yundrone_MOffice","ip":"192.168.10.198"}'
        rendered = status_cmd._format_wifi(True, text)
        self.assertEqual(rendered, "connected (SSID=Yundrone_MOffice)")

    def test_connected_ip_is_reported_separately(self) -> None:
        text = '{"state":"connected","ssid":"Yundrone_MOffice","ip":"192.168.10.198"}'
        rendered = status_cmd._format_ip(True, text)
        self.assertEqual(rendered, "192.168.10.198")

    def test_connected_ip_without_value_shows_unknown(self) -> None:
        text = '{"state":"connected","ssid":"Yundrone_MOffice","ip":""}'
        rendered = status_cmd._format_ip(True, text)
        self.assertEqual(rendered, "unknown")


if __name__ == "__main__":
    unittest.main()
