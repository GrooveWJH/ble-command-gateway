# BLE Wi-Fi Provisioning

[![中文](https://img.shields.io/badge/README-%E4%B8%AD%E6%96%87-red)](./README.md)

Provision a **Linux server's Wi-Fi over BLE** from a **cross-platform client** (macOS / Linux / Windows), with real-time status feedback (`Connecting` / `Success_IP` / `Fail`).

## Current Architecture (Refactored)

```text
.
├── app/                    # application entrypoints
│   ├── server_main.py
│   └── client_main.py
├── ble/                    # BLE gateway/runtime/publisher
├── protocol/               # envelope / codes / command ids
├── commands/               # registry/loader/builtin commands
├── services/               # provisioning + system command services
├── client/                 # interactive flow + command client
├── config/                 # UUID and default settings
├── server/                 # retained modules (preflight, link test)
├── scripts/                # legacy launcher
├── tools/                  # operations + legacy scripts
└── tests/                  # unit/integration/e2e
```

## BLE Protocol

- Service UUID: `6E400001-B5A3-F393-E0A9-E50E24DCCA9E`
- RX Characteristic (client -> server write): `6E400002-B5A3-F393-E0A9-E50E24DCCA9E`
- TX Characteristic (server -> client read/notify): `6E400003-B5A3-F393-E0A9-E50E24DCCA9E`

Request JSON example:

```json
{"id":"req-1","cmd":"provision","args":{"ssid":"LabWiFi","pwd":"secret"}}
```

## Built-in Commands

- `help`
- `ping`
- `status`
- `provision`
- `shutdown`
- `sys.whoami`
- `net.ifconfig`

`help` returns a short command list. Use `help` with `args.cmd` for detailed usage.

## Requirements

### Server (Linux)

- Ubuntu / Debian
- BlueZ
- NetworkManager (`nmcli`)
- Python `3.10-3.13`

```bash
sudo apt update
sudo apt install -y network-manager bluez
```

### Client (Cross-platform)

- macOS / Linux / Windows
- BLE hardware available
- Python `3.10-3.13`

## Dependency Management (uv)

```bash
uv sync --only-group server
uv sync --only-group client
```

## Quick Start

### 1) Start server (new entrypoint)

```bash
uv sync --only-group server
sudo -E "$(pwd)/.venv/bin/python" app/server_main.py \
  --device-name Orin_Drone_01 \
  --ifname wlan0 \
  --adapter hci0
```

### 2) Start client (new entrypoint)

```bash
uv sync --only-group client
"$(pwd)/.venv/bin/python" app/client_main.py --target-name Orin_Drone_01
```

## Phone Direct Debug

Write to RX characteristic from LightBlue / nRF Connect:

```json
{"id":"req-help-1","cmd":"help","args":{}}
```

Detailed help:

```json
{"id":"req-help-2","cmd":"help","args":{"cmd":"provision"}}
```

## Link Test

```bash
# server
sudo -E "$(pwd)/.venv/bin/python" tests/integration/server_link_test.py --adapter hci0

# client
"$(pwd)/.venv/bin/python" tests/integration/client_link_test.py \
  --target-name BLE_Hello_Server \
  --exchange-count 10 \
  --exchange-mode sequential
```

## Operations

Reset BLE runtime state:

```bash
sudo -E "$(pwd)/.venv/bin/python" tools/reset/server_reset.py --adapter hci0
```

`scripts/bless_uart.py` is a legacy demo launcher and refuses to run unless `--run-legacy` is explicitly provided.

## systemd

Use the new entrypoint in `ExecStart`:

```ini
ExecStart=/home/nvidia/ble-wifi-provisioning/.venv/bin/python /home/nvidia/ble-wifi-provisioning/app/server_main.py --device-name Orin_Drone_01 --ifname wlan0
```

## Validation

```bash
python3 -m py_compile app/server_main.py app/client_main.py ble/server_gateway.py
python3 -m unittest discover -s tests/unit -p 'test_*.py'
```

Real BLE hardware E2E validation is still required on your target device.
