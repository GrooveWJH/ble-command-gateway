# BLE Command Gateway

BLE provisioning and device diagnostics gateway: send Wi‑Fi commands to a Linux device over BLE and get observable status back.

- 中文说明: `README.md`
- Roadmap: `TODO.md`

## Overview

This project targets first-time provisioning, on-site Wi‑Fi switching, and headless diagnostics. It provides a scriptable BLE command channel plus an interactive client.

## Key Features

- Interactive client: scan, select device, reuse a long-lived session, Rich UI output
- Provisioning: send SSID/password (open networks supported), server executes via `nmcli` and returns final status + IP
- Diagnostics: `status / sys.whoami / net.ifconfig / wifi.scan` for on-site troubleshooting
- Observability: server logs and in-progress updates during provisioning; client-side wait/chunk receive visualization
- Extensibility: clear layering across protocol, command registry, system services, and BLE gateway

## Architecture

```text
app/        entrypoints (server_main.py / client_main.py)
ble/        BLE gateway/runtime/response publisher (chunking)
protocol/   protocol models/codec/status codes
commands/   command registry + built-in commands
services/   system execution + Wi‑Fi provisioning services
client/     scanning/session/interactive flow/rendering
config/     defaults and UUIDs
tests/      unit/integration tests
```

## BLE Protocol

- Service UUID: `6E400001-B5A3-F393-E0A9-E50E24DCCA9E`
- Client Write Char: `6E400002-B5A3-F393-E0A9-E50E24DCCA9E`
- Server Read/Notify Char: `6E400003-B5A3-F393-E0A9-E50E24DCCA9E`

Request JSON example:

```json
{"id":"req-1","cmd":"provision","args":{"ssid":"LabWiFi","pwd":"secret"}}
```

## Commands

- `help`
- `ping`
- `status`
- `provision`
- `shutdown`
- `sys.whoami`
- `net.ifconfig`
- `wifi.scan`

## Requirements

- Python: `3.10 - 3.13`
- Dependency manager: `uv`
- Server: Linux only (BlueZ + NetworkManager `nmcli`)

System deps (Debian/Ubuntu):

```bash
sudo apt update
sudo apt install -y network-manager bluez
```

Python deps:

```bash
uv sync --only-group server
uv sync --only-group client
```

## Quick Start

Server (Linux):

```bash
sudo -E "$(pwd)/.venv/bin/python" app/server_main.py \
  --device-name Yundrone_UAV \
  --ifname wlan0 \
  --adapter hci0 \
  --log-level INFO
```

Client (macOS/Linux/Windows):

```bash
"$(pwd)/.venv/bin/python" app/client_main.py --target-name Yundrone_UAV
```

## Suggested Flow

- Minimal link check: `help` -> `status` -> `wifi.scan` -> `provision`
- `status`: verify current SSID and IP
- `wifi.scan`: verify target SSID visibility and signal strength

## Tests

```bash
python3 -m py_compile app/server_main.py app/client_main.py
python3 -m unittest discover -s tests/unit -p 'test_*.py'
```

## Deployment

- Use `app/server_main.py` as systemd `ExecStart`
- Server is often run with `sudo`; `status/whoami` prefers the operator account (e.g. `SUDO_USER`)

Example:

```ini
ExecStart=/path/to/.venv/bin/python /path/to/app/server_main.py --device-name Yundrone_UAV --ifname wlan0 --adapter hci0
```

## Roadmap

- Link heartbeat and disconnect detection (see `TODO.md`)
- Finer-grained provisioning progress model
- More complete e2e automation and stress testing
