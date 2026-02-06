"""Project defaults and payload key constants."""

SSID_KEY = "ssid"
PASSWORD_KEY = "pwd"

STATUS_STANDBY = "Standby"
STATUS_CONNECTING = "Connecting"
STATUS_SUCCESS_PREFIX = "Success_IP:"
STATUS_FAIL_PREFIX = "Fail:"
STATUS_BUSY_PREFIX = "Busy:"

DEFAULT_DEVICE_NAME = "Yundrone_UAV"
DEFAULT_SCAN_TIMEOUT = 25
DEFAULT_WAIT_TIMEOUT = 45
DEFAULT_CONNECT_TIMEOUT = 40

# Client connect retry when selecting device
DEFAULT_CONNECT_RETRIES = 1

# Max BLE payload size to avoid MTU truncation (bytes).
# Keep conservative for cross-platform BLE stacks.
MAX_BLE_PAYLOAD_BYTES = 360

# BLE advertising fast-start settings (BlueZ experimental options).
# Interval unit is milliseconds (range 20ms+ when supported by BlueZ).
FAST_ADV_INTERVAL_MS = 25
FAST_ADV_DURATION_SEC = 300
