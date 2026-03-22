# BLE Command Gateway (Rust Port)

[![中文版](https://img.shields.io/badge/README-中文-blue)](./README.md)

**BLE Provisioning and Diagnostics Gateway**: Built entirely with memory-safe Rust. This repository provides a complete open-source solution for dispatching Wi-Fi provisioning credentials over low-energy Bluetooth (BLE) to headless devices (Linux / Raspberry Pi / Jetson Orin), alongside extracting network and system diagnostic probes.

The legacy `Python + bluedot / PySimpleGUI` prototype has been **100% fully rewritten into extensively modularized Rust**. This massive overhaul brings i18n support natively to the binaries, seamless automated MTU chunking logic for transferring huge JSON structures across air-gaps, and a native asynchronous thread-pool engine.

---

## 🏗️ New Workspace Architecture

The Cargo Workspace is strictly decoupled into four highly cohesive domains:

1. **`protocol` (Core Data Layer)** (`crates/protocol`)
   - **Zero-dependency** algorithm algorithms.
   - `commands.rs`: The global command routing dictionary.
   - `chunking.rs`: A custom sub-layer protocol designed to split massive payloads into ~360 Bytes to safely navigate around low BLE MTU hardware limitations, seamlessly reassembling them down the pipeline.

2. **`server` (Linux Embedded Peripheral)** (`crates/server`)
   - *(Linux ARM/x86 compilation target ONLY)*
   - `main.rs`: Interfaces directly with BlueZ D-Bus APIs to broadcast as a peripheral acting as `Yundrone_UAV` with native custom GATT characteristics.
   - `services.rs`: Utilizes `tokio::process` to asynchronously execute headless OS bindings like `nmcli device wifi connect`, `ifconfig`, and `whoami`.

3. **`client` (Cross-Platform CLI)** (`crates/client`)
   - `ble.rs`: Native multi-os BLE Central device connector built atop `btleplug`.
   - `main.rs`: Highly interactive Command-Line Interface. Renders system diagnostics in terminal tables (`comfy-table`), offers hidden secure password inputs (`inquire`), and ships with a lightweight i18n translator (`--lang en`).

4. **`gui` (Decoupled Native GUI App)** (`crates/gui`)
   - `main.rs`: Extremely minimal 34-line integration boundary.
   - `i18n.rs`: Zero-dependency internationalization dictionary allowing hot swaps.
   - `ble_worker.rs`: A background `tokio` observer thread hiding all heavy-lifting IO routines and delivering robust MPSC asynchronous feedback to the visual components.
   - `app.rs`: The high-refresh-rate `egui` native canvas with beautiful replicas of the "Wi-Fi Provisioning", "Diagnostics", and "Raw Logs" control panels.

---

## 🚀 Quickstart

A standard Rust toolchain (`rustup`) is required.

### Launch GUI Native Client (Mac / Win / Linux)
```bash
cargo run -p gui
```

### Launch Interactive Safe Terminal Interface
Pass the localized `--lang` attribute per your needs (defaults to `zh`).
```bash
cargo run -p client -- --lang en 
```

### Build Embedded Daemon Target (Execute on Raspberry Pi / Jetson Linux)
```bash
cargo build --release -p server
# The high-performance footprint binary surfaces at target/release/server
```

For deployment via `systemd` across autonomous robots, see [docs/systemd.md](./docs/systemd.md).

If you are an engineer looking to embed new custom robotic hook endpoints, see [docs/COMMAND_AUTHORING.md](./docs/COMMAND_AUTHORING.md).
