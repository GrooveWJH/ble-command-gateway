# BLE Wi-Fi Provisioning

[![中文](https://img.shields.io/badge/README-%E4%B8%AD%E6%96%87-red)](./README.md)

Provision a **Linux server's Wi-Fi over BLE** from a **cross-platform client** (macOS / Linux / Windows), with real-time status feedback (`Connecting` / `Success_IP` / `Fail`).

## Features

- Cross-platform client (`bleak` + `InquirerPy`)
- Linux-only server (`bless` + BlueZ + `nmcli`)
- Persistent interactive client session (menu loop)
- Clear terminal states and exit codes
- `systemd` deployment support

## Repository Layout

```text
.
├── client/
│   └── client_config_tool.py
├── server/
│   └── wifi_ble_service.py
├── config.py
├── sync_to_orin.sh
├── pyproject.toml
├── README.md
└── README_EN.md
```

## BLE Protocol

- Service UUID: `A07498CA-AD5B-474E-940D-16F1FBE7E8CD`
- Write Characteristic (client -> server): `51FF12BB-3ED8-46E5-B4F9-D64E2FEC021B`
- Read/Notify Characteristic (server -> client): `51FF12BB-3ED8-46E5-B4F9-D64E2FEC021C`

Write payload:

```json
{"ssid": "LabWiFi", "pwd": "secret"}
```

## Requirements

### Server (Linux)

- Ubuntu / Debian
- BlueZ
- NetworkManager (`nmcli`)
- Python `3.10-3.13`

System packages:

```bash
sudo apt update
sudo apt install -y network-manager bluez
```

### Client (Cross-platform)

- macOS / Linux / Windows
- BLE hardware available
- Python `3.10-3.13`

## Dependency Management (uv)

Groups:

- `server`: `bless`, `dbus-next`
- `client`: `bleak`, `inquirerpy`
- Compatibility note: server is pinned to `bless==0.3.0`, while client `bleak` is resolved by `uv` (currently can resolve to 2.x).

Install:

```bash
uv sync --only-group server
uv sync --only-group client
```

## Quick Start

### 1) Start server on Linux target

```bash
uv sync --only-group server
sudo uv run --no-sync python server/wifi_ble_service.py \
  --device-name Orin_Drone_01 \
  --ifname wlan0
```

### 2) Start interactive client menu

```bash
uv sync --only-group client
uv run --no-sync python client/client_config_tool.py --target-name Orin_Drone_01
```

## Client Interactive Menu

The client stays alive and lets users repeat operations:

- Scan and select device
- Update device-name filter
- Set Wi-Fi credentials
- Provision selected device
- One-shot flow (scan -> input -> provision)
- Show session state
- Exit

## Client Exit Codes

- `0`: success
- `2`: device not found
- `3`: provisioning failed / BLE interaction error
- `4`: timeout waiting terminal state
- `5`: input error

## systemd Deployment (Server)

Create `/etc/systemd/system/drone-ble.service`:

```ini
[Unit]
Description=Drone BLE Provisioning Service
After=bluetooth.target network.target

[Service]
Type=simple
WorkingDirectory=/home/nvidia/ble-wifi-provisioning
ExecStart=/home/nvidia/ble-wifi-provisioning/.venv/bin/python /home/nvidia/ble-wifi-provisioning/server/wifi_ble_service.py --device-name Orin_Drone_01 --ifname wlan0
User=root
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable:

```bash
cd /home/nvidia/ble-wifi-provisioning
uv sync --only-group server
sudo systemctl daemon-reload
sudo systemctl enable drone-ble.service
sudo systemctl start drone-ble.service
sudo systemctl status drone-ble.service
```

## Sync to Remote

```bash
./sync_to_orin.sh
```

Default target: `orin-Mocap5G:~/work/ble-wifi-provisioning/`

## Validation Status

Validated locally:

- `uv run ruff check .`
- `python3 -m py_compile config.py server/wifi_ble_service.py client/client_config_tool.py`

BLE hardware end-to-end behavior still requires real-device validation.
