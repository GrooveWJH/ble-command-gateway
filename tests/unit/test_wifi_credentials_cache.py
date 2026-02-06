from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from client import interactive_flow


class WifiCredentialsCacheTests(unittest.TestCase):
    def test_load_returns_none_when_missing(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            with patch.dict("client.interactive_flow.os.environ", {"XDG_CACHE_HOME": td}, clear=False):
                ssid, pwd = interactive_flow.load_cached_wifi_credentials()
        self.assertIsNone(ssid)
        self.assertIsNone(pwd)

    def test_save_and_load_roundtrip(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            with patch.dict("client.interactive_flow.os.environ", {"XDG_CACHE_HOME": td}, clear=False):
                ok = interactive_flow.save_wifi_credentials("LabWiFi", "secret")
                self.assertTrue(ok)
                ssid, pwd = interactive_flow.load_cached_wifi_credentials()
                self.assertEqual(ssid, "LabWiFi")
                self.assertEqual(pwd, "secret")

                cache_file = Path(td) / "ble-command-gateway" / "wifi_credentials.json"
                self.assertTrue(cache_file.exists())


if __name__ == "__main__":
    unittest.main()
