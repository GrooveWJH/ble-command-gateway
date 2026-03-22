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

### 2. Run the Command-Line Client
Starts an interactive terminal UI for headless control.
```bash
cargo run -p client -- --lang en 
```

### 3. Build the Server Daemon (Linux Only)
Builds the BLE peripheral server that listens for incoming commands.
```bash
cargo build --release -p server
```
For instructions on registering the server as a system service, see [docs/systemd.md](./docs/systemd.md).

## Project Structure

The repository is organized as a Cargo Workspace with four main crates:

- `protocol/`: Core data structures, chunking algorithm, and command definitions. No external dependencies.
- `server/`: Linux BLE peripheral implementation handling incoming requests and system executions (`nmcli`).
- `client/`: Cross-platform BLE central connection library utilizing `btleplug`.
- `gui/`: Native user interface built with `egui` and a background `tokio` worker thread.

## Documentation

- Extending commands: [COMMAND_AUTHORING.md](./docs/COMMAND_AUTHORING.md)
- Development roadmap: [TODO.md](./TODO.md)

## License

MIT License.
