"""Wi-Fi credential cache helpers for CLI and GUI callers."""

from __future__ import annotations

import json
import os
from pathlib import Path

from config.defaults import PASSWORD_KEY, SSID_KEY


def cache_path() -> Path:
    cache_home_raw = os.environ.get("XDG_CACHE_HOME")
    cache_home = Path(cache_home_raw) if cache_home_raw else (Path.home() / ".cache")
    return cache_home / "ble-command-gateway" / "wifi_credentials.json"


def load_cached_wifi_credentials() -> tuple[str | None, str | None]:
    path = cache_path()
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
    path = cache_path()
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
