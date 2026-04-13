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
- **Multi-Device Friendly Naming**: The server advertises a dynamic device name in the form `Yundrone_UAV-HH-MM-ABCD`, so multiple devices booting at the same time remain distinguishable.
- **Memory Safety**: Built with Rust and `tokio` to ensure safe, concurrent handling of Bluetooth I/O and UI rendering.

## Installation And Deployment

This project has two roles:

- `server`: runs on the target Linux device and advertises itself over BLE
- `client` / `gui`: runs on your laptop or workstation and connects to the server

The safest order is:

1. Install the Rust toolchain
2. Build and deploy the Linux `server`
3. Build and run either the `client` CLI or the `gui`

### 1. Install the Rust toolchain

#### macOS

Install Apple's command-line developer tools first:

```bash
xcode-select --install
```

Then install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

#### Ubuntu / Debian

Install native build dependencies first:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libdbus-1-dev libudev-dev
```

Then install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

#### Windows

Install Visual Studio Build Tools with the C++ toolchain first, then install Rust:

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

The `server` crate is Linux-only. It is intended for the headless device that will receive Wi-Fi credentials and status commands over BLE.

#### 3.1 Install Linux runtime dependencies on the target device

At minimum, the target device needs:

- BlueZ / `bluetoothd`
- `NetworkManager` and `nmcli`
- `pkg-config` and `libdbus-1-dev` if you build on-device

Example on Ubuntu-based devices:

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

The resulting binary will be:

```text
target/release/server
```

#### 3.3 Test the server manually before systemd

```bash
sudo ./target/release/server
```

When startup succeeds, the log should include:

- `ble.server.starting`
- `ble.advertising.ready`
- `ble.gatt.ready`

The log also prints the real advertised BLE name, for example:

```text
Yundrone_UAV-15-19-A7F2
```

#### 3.4 Install the systemd service

The repository ships a ready-to-edit unit file:

```text
deploy/systemd/ble-command-gateway.service
```

Typical install flow:

```bash
sudo cp deploy/systemd/ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ble-command-gateway.service
```

Check service status and logs:

```bash
sudo systemctl status ble-command-gateway.service --no-pager
sudo journalctl -u ble-command-gateway.service -f
```

For a full walkthrough, see [docs/systemd.md](./docs/systemd.md).

### 4. Install and run the client

The project provides two client-side entrypoints:

- `gui`: graphical desktop app for scanning, provisioning, diagnostics, and logs
- `client`: interactive CLI for terminal-driven workflows

You can build both with:

```bash
cargo build --release -p gui -p client
```

#### 4.1 Run the GUI

Development run:

```bash
cargo run -p gui
```

Release run:

```bash
./target/release/gui
```

Usage flow:

1. Enter the stable prefix `Yundrone_UAV`
2. Click scan
3. Select the exact BLE instance from the candidate list
4. Use the provisioning and diagnostic panels after the connection is established

On macOS, the GUI will relaunch itself through a signed `.app` wrapper so Bluetooth permissions are requested through a proper app bundle.

#### 4.2 Run the CLI

Development run:

```bash
cargo run -p client -- --lang en
```

Release run:

```bash
./target/release/client --lang en
```

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

### Linux server: advertising interval may be ignored on some BlueZ setups

On some Linux devices, the server may log a fast advertising interval such as `20 ms`, while the controller still ends up advertising at a much slower default interval.

This was observed on an `OrangePi 4 Pro` running:

- BlueZ `5.64`
- `bluetoothd` started without `--experimental`

In that setup, BlueZ accepted `MinInterval` / `MaxInterval` on the D-Bus advertisement object, but silently ignored them before forwarding advertising parameters to mgmt/HCI. After enabling:

```ini
ExecStart=/usr/lib/bluetooth/bluetoothd --experimental
```

the requested advertising interval was correctly propagated to mgmt/HCI on that machine.

Important:

- This is documented here as a possible platform-specific issue, not as a guaranteed problem on every Linux device.
- If you see server logs claiming `20 ms` but discovery still feels unusually slow, inspect the real HCI parameters instead of trusting the application log alone.

Recommended workaround on affected machines:

1. Add a systemd override for `bluetooth.service` so `bluetoothd` starts with `--experimental`
2. Restart `bluetooth.service`
3. Restart `ble-command-gateway.service`
4. Verify with `btmon` that `LE Set Extended Advertising Parameters` now shows the requested interval

Example override:

```ini
[Service]
ExecStart=
ExecStart=/usr/lib/bluetooth/bluetoothd --experimental
```

## License

MIT License.
