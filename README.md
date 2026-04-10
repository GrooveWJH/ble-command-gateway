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

## Quick Start

### Prerequisites
- Rust toolchain (`cargo`, `rustup`)
- A supported Bluetooth adapter
- Linux environment for the server daemon

### 1. Run the GUI Client (Mac/Win/Linux)
Starts a graphical interface for scanning devices, provisioning Wi-Fi, and viewing logs.
```bash
cargo run -p gui
```
Enter the stable prefix `Yundrone_UAV`, scan, then choose the exact advertised instance from the candidate list.
On macOS, the GUI runtime will relaunch itself through a signed `.app` wrapper so Bluetooth permissions are handled by a proper app bundle.

### 2. Run the Command-Line Client
Starts an interactive terminal UI for headless control.
```bash
cargo run -p client -- --lang en 
```
The CLI scans by prefix, prints all matching BLE instances with RSSI, and prompts you to select the exact target when more than one match is found.
On macOS, the CLI now goes through the same shared runtime compatibility layer before touching CoreBluetooth.

### 3. Build the Server Daemon (Linux Only)
Builds the BLE peripheral server that listens for incoming commands.
```bash
cargo build --release -p server
```
For instructions on registering the server as a system service, see [docs/systemd.md](./docs/systemd.md).

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
