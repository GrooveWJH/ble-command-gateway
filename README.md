# YunDrone BLE Gateway

[![中文版](https://img.shields.io/badge/README-中文-blue?style=flat-square)](./README_ZH.md)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?style=flat-square&logo=rust)](#)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](#)

YunDrone BLE Gateway lets a phone, desktop app, or CLI configure and diagnose a headless Linux device over Bluetooth Low Energy. It is built for the moment when the target device has no display, no keyboard, and no working network yet.

The gateway can scan nearby Wi-Fi networks, provision credentials, read system status, and manage saved Wi-Fi profiles. The Linux device runs the BLE server. Your workstation runs the GUI or CLI client.

## Choose Your Path

| Goal | Start here | Notes |
| --- | --- | --- |
| Choose server deployment or client launch from one TUI | `bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh)` | Recommended entry. Downloads Gum, caches the client CLI binary, and delegates server deployment. |
| Use the desktop app on macOS | Download the macOS release asset | Current official prebuilt asset is Apple Silicon only. |
| Run from source on your workstation | Build `gui` or `yundrone-ble-client` | Best for development and debugging. |
| Deploy the BLE server on Linux | Run the unified entry or server-only entry | Target device needs BlueZ and NetworkManager. |
| Debug a BLE link | Run `yundrone-ble-client debug-ble` | Shows scan, connect, GATT discovery, V2 transport frames, reassembly, and ACKs. |

## Quick Use

Recommended one-command entry:

```bash
bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh)
```

1. On the Linux target device, choose “Deploy / manage local BLE Server”.
2. On your workstation, choose “Launch BLE Client CLI”, or open the GUI.
3. Scan for a BLE local name starting with `yundrone-`, for example `yundrone-ytcwln`.
4. Connect to the matching device.
5. Run Wi-Fi scan, Wi-Fi provision, system status, or saved Wi-Fi profile actions.
6. If something looks wrong, use the debug CLI before changing the server.

The project uses one public BLE name per device. The name is persisted in `/var/lib/yundrone/ble-device-name`, so restarting the server should not create a new identity and confuse mobile BLE caches.

## Server Deployment

The server package is `yundrone-ble-server`. It is intended for Linux target devices such as ARM development boards, Jetson, Raspberry Pi, or similar edge computers.

Recommended deployment entry:

```bash
bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh)
```

If you only want the server installer, the compatibility entry remains available:

```bash
bash <(curl -fsSL https://install.yundrone.cn/ble-server.sh)
```

Install runtime and build dependencies on Ubuntu or Debian:

```bash
sudo apt update
sudo apt install -y bluetooth bluez network-manager pkg-config libdbus-1-dev
```

Install Rust if the target device builds from source:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
rustup toolchain install 1.95.0 --profile minimal --component rustfmt --component clippy
```

Build and smoke-test the server:

```bash
cargo build --release -p yundrone-ble-server
sudo ./target/release/yundrone-ble-server
```

Healthy startup logs should include:

```text
ble.server.starting
ble.gatt.ready
ble.advertising.ready
identity_name=yundrone-...
```

Install the systemd service:

```bash
sudo chmod +x deploy/systemd/prepare-ble-adapter.sh
sudo cp deploy/systemd/yundrone-ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now yundrone-ble-command-gateway.service
```

Check status and readable logs:

```bash
sudo systemctl status yundrone-ble-command-gateway.service --no-pager
sudo journalctl -u yundrone-ble-command-gateway.service -f -o cat
```

Production installs run a systemd-managed release binary. Current field deployments use `/opt/yundrone/ble-command-gateway/current/yundrone-ble-server`; the repository systemd template also supports source builds under `/opt/ble-command-gateway`. Full deployment details, BlueZ settings, pairing policy, advertising interval checks, and recovery steps live in [docs/systemd.md](./docs/systemd.md).

## Client And GUI

Use the unified entry to download and launch the raw client CLI binary:

```bash
bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh) -- client
```

The client binary distribution targets are `macos-arm64`, `linux-amd64`, and `linux-arm64`. The macOS CLI distribution intentionally does not use an `.app` bundle.

Build the desktop GUI and CLI from source:

```bash
cargo build --release -p gui -p yundrone-ble-client
```

Run the GUI during development:

```bash
cargo run -p gui
```

Package a double-clickable macOS app:

```bash
chmod +x scripts/package-macos-gui.sh
./scripts/package-macos-gui.sh
open "target/release/yundrone-ble-client.app"
```

Run the interactive CLI:

```bash
cargo run -p yundrone-ble-client -- interactive --lang en
```

Run the release CLI directly:

```bash
./target/release/yundrone-ble-client interactive --lang en
```

CLI help is explicit and subcommand-based:

```bash
cargo run -p yundrone-ble-client -- --help
cargo run -p yundrone-ble-client -- interactive --help
cargo run -p yundrone-ble-client -- debug-ble --help
```

When using `cargo run`, the `--` separates Cargo arguments from program arguments. When running `./target/release/yundrone-ble-client` directly, do not include that separator.

## Debug And Development

Use the debug CLI when a device is hard to find, connection is slow, services do not appear, or responses look truncated:

```bash
cargo run -p yundrone-ble-client -- debug-ble \
  --target yundrone \
  --timeout 30 \
  --response-timeout 15 \
  --trace-chunks \
  --trace-qos \
  --output /tmp/yundrone-ble-debug.log
```

With `--trace-chunks` and `--trace-qos`, the log shows the BLE transport rather than only the business JSON. Current clients use a compact V2 binary frame so every conservative 20-byte BLE write/notify carries a 4-byte header and up to 16 bytes of request or response payload:

- `[TX:packet]` / `[RX:packet]`: each V2 transport packet, including `RequestChunk`, `RequestFinal`, `ResponseChunk`, `ResponseFinal`, one-frame `Progress`, `AckRange`, or `AckEvent`.
- `[RX:transport]`: decoded transport metadata such as stream id, frame index, final flag, and payload length.
- `[RX:assembled]`: the fully reassembled response JSON.
- `[QOS:tx]`: request or ACK writes sent by the client.
- `[OK] rx`: the decoded final response summary.

Large final results such as `wifi.scan` can print many packets because each result event is split into 16-byte payload frames and then reassembled. Periodic in-progress ticks are single header-only `Progress` control frames. The older JSON `data.chunk` / `link.ack` path is still documented for compatibility and manual BLE debugger fallback, but the normal CLI path is V2 compact transport.

Common local checks:

```bash
scripts/ci/check.sh quality
```

This script is the same entry point used by GitHub Actions for formatting, tests, clippy, release script tests, and version checks. The Rust version is pinned by [rust-toolchain.toml](./rust-toolchain.toml), so local checks and CI use the same toolchain. Build parity commands are also available: `scripts/ci/check.sh build-full`, `scripts/ci/check.sh build-desktop`, and `scripts/ci/check.sh package-macos`.

Release versioning is driven by [VERSION](./VERSION) and [CHANGELOG](./CHANGELOG). Tagged releases use the release workflow to publish the macOS app asset; the install service is synchronized manually with `scripts/release/*` for `ble-wifi-tool.sh`, the server installer, raw client binaries, and tools.

## Documentation Map

- Server deployment and systemd operations: [docs/systemd.md](./docs/systemd.md)
- BLE debugger guide with JSON commands: [docs/BLE_DEBUGGER_GUIDE_ZH.md](./docs/BLE_DEBUGGER_GUIDE_ZH.md) (Chinese)
- Protocol command contracts: [docs/COMMANDS.md](./docs/COMMANDS.md)
- Compatible BLE server implementation guide: [docs/COMPATIBLE_BLE_SERVER_IMPLEMENTATION_ZH.md](./docs/COMPATIBLE_BLE_SERVER_IMPLEMENTATION_ZH.md) (Chinese)
- BLE MTU chunking middleware: [docs/MTU_CHUNKING_ZH.md](./docs/MTU_CHUNKING_ZH.md) (Chinese)
- WeChat Mini Program discovery guide: [docs/WECHAT_MINIPROGRAM_DISCOVERY_ZH.md](./docs/WECHAT_MINIPROGRAM_DISCOVERY_ZH.md) (Chinese)
- Rust client library API: [docs/LIBRARY_API.md](./docs/LIBRARY_API.md)
- Command extension guide: [docs/COMMAND_AUTHORING.md](./docs/COMMAND_AUTHORING.md)

## Release And Platform Support

GitHub Releases currently provide one official prebuilt asset:

- macOS Apple Silicon: `yundrone-ble-client-macos-arm64.zip`, containing `yundrone-ble-client.app`.

Platform status:

- macOS: official prebuilt GUI app is attached to tagged releases.
- macOS / Linux: `install.yundrone.cn` distributes raw client CLI binaries for `macos-arm64`, `linux-amd64`, and `linux-arm64`.
- Linux: the server is deployed as a systemd service through the unified or server-only installer.
- Windows: CI validates desktop builds; no official prebuilt binary is attached yet.

## Project Layout

This repository is a Cargo workspace:

- `crates/protocol`: wire schema, typed requests/responses, V2 BLE transport framing, and legacy response chunking.
- `crates/server`: Linux BLE peripheral and NetworkManager integration. Cargo package and binary: `yundrone-ble-server`.
- `crates/client`: BLE central library and CLI. Cargo package and binary: `yundrone-ble-client`.
- `crates/gui`: native desktop GUI built with `egui`.
- `crates/platform_runtime`: platform launch helpers, mainly for macOS app-bundle behavior.

Legacy Python service entrypoints have been removed. Current deployment uses the Rust server.

## License

MIT License.
