# YunDrone BLE Gateway

[![中文版](https://img.shields.io/badge/README-中文-blue?style=flat-square)](./README_ZH.md)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?style=flat-square&logo=rust)](#)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](#)

A Bluetooth Low Energy (BLE) gateway for provisioning and diagnosing headless Linux devices (e.g., Raspberry Pi, Jetson). 

This project allows you to send Wi-Fi credentials and retrieve system status over a BLE connection, bypassing the need for an existing network infrastructure. It is written in Rust and operates across multiple platforms.

## Features

- **Protocol Chunking**: Implements a custom chunking algorithm to reliably transmit large JSON payloads over BLE MTU limits (~360 Bytes).
- **Headless Server**: The Linux-based server daemon (`bluer` + `nmcli`) runs as a background process to handle incoming Wi-Fi credentials and system commands.
- **Cross-Platform Client**: Provides both a terminal UI (CLI) and a native graphical interface (`egui`) for connecting to the server.
- **Discoverability-First Identity**: The server uses a short primary BLE name such as `YD-A3FB` for reliable discovery, while preserving the full dynamic instance name `Yundrone_UAV-HH-MM-ABCD` in logs and post-connect context.
- **Memory Safety**: Built with Rust and `tokio` to ensure safe, concurrent handling of Bluetooth I/O and UI rendering.

## Installation And Deployment

This project has two roles: `server` on the target Linux device, and `client` / `gui` on your workstation. Recommended order: install Rust, deploy the Linux `server`, then run `client` or `gui`.

### 1. Install the Rust toolchain

#### macOS

Install Apple's command-line developer tools, then Rust:

```bash
xcode-select --install
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

#### Ubuntu / Debian

Install native build dependencies, then Rust:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libdbus-1-dev libudev-dev
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

#### Windows

Install Visual Studio Build Tools with the C++ toolchain, then Rust:

```powershell
winget install Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc
rustup component add rustfmt clippy
rustc --version
cargo --version
```

### 2. Clone the repository

```bash
git clone https://github.com/GrooveWJH/ble-command-gateway.git
cd ble-command-gateway
```

### 3. Deploy the Linux server

The `server` crate is Linux-only and is intended for the headless device that will receive Wi-Fi credentials and status commands over BLE.

#### 3.1 Install Linux runtime dependencies on the target device

At minimum, the target device needs BlueZ / `bluetoothd`, `NetworkManager` / `nmcli`, and if you build on-device, `pkg-config` plus `libdbus-1-dev`. Example on Ubuntu-based devices:

```bash
sudo apt update
sudo apt install -y bluetooth bluez network-manager pkg-config libdbus-1-dev
```

#### 3.2 Build the server

If you build directly on the target device:

```bash
source "$HOME/.cargo/env"
cargo build --release -p server
```

The resulting binary is `target/release/server`.

#### 3.3 Test the server manually before systemd

```bash
sudo ./target/release/server
```

When startup succeeds, the log should include `ble.server.starting`, `ble.advertising.ready`, and `ble.gatt.ready`. It also prints both the full instance identity and the short primary BLE name, for example:

```text
advertised_name=Yundrone_UAV-15-19-A7F2 short_name=YD-A7F2
```

#### 3.4 Install the systemd service

The repository ships a ready-to-edit unit file:

```text
deploy/systemd/yundrone-ble-command-gateway.service
```

Typical install flow:

```bash
sudo cp deploy/systemd/yundrone-ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now yundrone-ble-command-gateway.service
```

Check service status and logs:

```bash
sudo systemctl status yundrone-ble-command-gateway.service --no-pager
sudo journalctl -u yundrone-ble-command-gateway.service -f
```

For a full walkthrough, see [docs/systemd.md](./docs/systemd.md).

### 4. Install and run the client

The project provides two client-side entrypoints: `gui` for the desktop app, and `client` for the interactive CLI. Build both with:

```bash
cargo build --release -p gui -p client
```

#### 4.1 Run the GUI

Run the GUI in development with `cargo run -p gui`, or in release with `./target/release/gui`.

Package a double-clickable macOS app bundle:

```bash
chmod +x scripts/package-macos-gui.sh
./scripts/package-macos-gui.sh
open "target/release/YunDrone BLE Gateway.app"
```

This builds the release GUI binary, stages a proper `.app` bundle, copies the project `Info.plist`, and applies an ad-hoc signature so Finder can launch it as a normal macOS app.

Usage flow:

1. Enter the stable prefix `Yundrone_UAV`
2. Click scan
3. Select the matching `YD-*` or full `Yundrone_UAV-*` candidate from the list
4. Use the provisioning and diagnostic panels after the connection is established

On macOS, the GUI will relaunch itself through a signed `.app` wrapper so Bluetooth permissions are requested through a proper app bundle.

#### 4.2 Run the CLI

Run the CLI in development with `cargo run -p client -- --lang en`, or in release with `./target/release/client --lang en`.

The CLI scans by prefix, lists every matching BLE instance with RSSI, and lets you choose the exact device before connecting.

On macOS, the CLI uses the same shared runtime compatibility layer before touching CoreBluetooth.

### 5. Recommended verification after deployment

After both sides are installed:

1. Start the Linux `server`
2. Open the `gui` or `client`
3. Scan with the prefix `Yundrone_UAV`
4. Confirm you can see the full advertised instance name
5. Connect and run:
   - Wi-Fi scan
   - status
   - ping
6. Confirm the server log prints matching `request_id` and response events

## Project Structure

The repository is organized as a Cargo Workspace with five main crates:

- `protocol/`: Core data structures, chunking algorithm, and command definitions. No external dependencies.
- `platform_runtime/`: Shared launch-preparation layer for macOS bundle staging plus Linux/Windows no-op shims.
- `server/`: Linux BLE peripheral implementation handling incoming requests and system executions (`nmcli`).
- `client/`: Cross-platform BLE central connection library utilizing `btleplug`.
- `gui/`: Native user interface built with `egui` and a background `tokio` worker thread.

## Documentation

- Extending commands: [COMMAND_AUTHORING.md](./docs/COMMAND_AUTHORING.md)
- Command response contracts: [COMMANDS.md](./docs/COMMANDS.md)
- Rust client library API: [LIBRARY_API.md](./docs/LIBRARY_API.md)
- Development roadmap: [TODO.md](./TODO.md)

Legacy Python service entrypoints have been removed. The repository no longer ships Python runtime code for deployment.

## Troubleshooting

### Linux server: discoverability-first advertising requires `bluetoothd --experimental`

The Linux server now assumes a discoverability-first BLE profile:

- fast-start interval: `25 ms`
- fast-start duration: `300 s`
- steady interval: `152.5 ms`
- short primary advertising name: `YD-XXXX`

This was observed on an `OrangePi 4 Pro` running:

- BlueZ `5.64`
- `bluetoothd` started without `--experimental`

In that setup, BlueZ accepted `MinInterval` / `MaxInterval` on the D-Bus advertisement object, but silently ignored them before forwarding advertising parameters to mgmt/HCI. After enabling:

```ini
ExecStart=/usr/lib/bluetooth/bluetoothd --experimental
```

the requested advertising interval was correctly propagated to mgmt/HCI on that machine.

Important:

- This is a deployment requirement for the discoverability-first profile.
- If you see server logs claiming `25 ms` but discovery still feels unusually slow, inspect the real HCI parameters instead of trusting the application log alone.
- Do not rely on the full dynamic instance name being present in scan response; the short primary name is the reliable on-air identity.

Recommended workaround on affected machines:

1. Add a systemd override for `bluetooth.service` so `bluetoothd` starts with `--experimental`
2. Restart `bluetooth.service`
3. Restart `yundrone-ble-command-gateway.service`
4. Verify with `btmon` that `LE Set Extended Advertising Parameters` now shows the requested interval
5. Verify that the primary advertising name is the short identity, such as `YD-A3FB`

Example override:

```ini
[Service]
ExecStart=
ExecStart=/usr/lib/bluetooth/bluetoothd --experimental
```

## License

MIT License.
