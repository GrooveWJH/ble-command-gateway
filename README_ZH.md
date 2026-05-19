# YunDrone BLE Gateway

[![English](https://img.shields.io/badge/README-English-blue?style=flat-square)](./README.md)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?style=flat-square&logo=rust)](#)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](#)

YunDrone BLE Gateway 用低功耗蓝牙连接一台还没有网络、没有显示器、没有键盘的 Linux 设备。你可以用手机、桌面 GUI 或 CLI 给它配 Wi-Fi、查看系统状态，并管理已经保存过的 Wi-Fi 记忆。

目标 Linux 设备运行 BLE server。你的电脑运行 GUI 或 CLI client。

## 选择你的入口

| 目标 | 从这里开始 | 说明 |
| --- | --- | --- |
| 直接使用 macOS 桌面程序 | 下载 GitHub Release 里的 macOS 包 | 当前官方预编译包只提供 Apple Silicon 版本。 |
| 在电脑上从源码运行 | 构建 `gui` 或 `yundrone-ble-client` | 适合开发、调试和日常验证。 |
| 在 Linux 设备上部署 BLE 服务 | 构建 `yundrone-ble-server` 并安装 systemd 服务 | 目标设备需要 BlueZ 和 NetworkManager。 |
| 排查蓝牙链路 | 运行 `yundrone-ble-client debug-ble` | 会展示扫描、连接、GATT 发现、notify 数据和可选分片。 |

## 快速使用

1. 在目标 Linux 设备上启动 server。
2. 在电脑上打开 GUI 或 CLI。
3. 扫描以 `yundrone-` 开头的 BLE local name，例如 `yundrone-ytcwln`。
4. 选择对应设备并连接。
5. 连接后执行 Wi-Fi 扫描、Wi-Fi 配网、系统状态或已保存 Wi-Fi 管理。
6. 如果设备难找、连接慢、服务列表不出现或响应像被截断，先跑 debug CLI。

每台设备只使用一个公开 BLE 名字。这个名字保存在 `/var/lib/yundrone/ble-device-name`，所以正常重启 server 不会换名字，也不容易污染手机和调试工具的蓝牙缓存。

## Server 部署

服务端 package 名是 `yundrone-ble-server`。它部署在 Linux ARM 开发板、Jetson、树莓派或类似边缘 Linux 设备上。

在 Ubuntu / Debian 系目标设备上安装运行和构建依赖：

```bash
sudo apt update
sudo apt install -y bluetooth bluez network-manager pkg-config libdbus-1-dev
```

如果目标设备需要本机编译，安装 Rust：

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
rustup toolchain install 1.95.0 --profile minimal --component rustfmt --component clippy
```

构建并手动冒烟测试 server：

```bash
cargo build --release -p yundrone-ble-server
sudo ./target/release/yundrone-ble-server
```

健康启动时，日志至少应包含：

```text
ble.server.starting
ble.gatt.ready
ble.advertising.ready
identity_name=yundrone-...
```

安装 systemd 服务：

```bash
sudo chmod +x deploy/systemd/prepare-ble-adapter.sh
sudo cp deploy/systemd/yundrone-ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now yundrone-ble-command-gateway.service
```

查看状态和适合人读的日志：

```bash
sudo systemctl status yundrone-ble-command-gateway.service --no-pager
sudo journalctl -u yundrone-ble-command-gateway.service -f -o cat
```

生产部署时，systemd 服务运行 `/opt/ble-command-gateway/target/release/yundrone-ble-server`。完整部署细节、BlueZ 设置、配对策略、广播 interval 验收和故障恢复请看 [docs/systemd.md](./docs/systemd.md)。

## Client 和 GUI

从源码构建桌面 GUI 和 CLI：

```bash
cargo build --release -p gui -p yundrone-ble-client
```

开发态运行 GUI：

```bash
cargo run -p gui
```

打包成可以在 macOS Finder 双击启动的 `.app`：

```bash
chmod +x scripts/package-macos-gui.sh
./scripts/package-macos-gui.sh
open "target/release/yundrone-ble-client.app"
```

运行交互式 CLI：

```bash
cargo run -p yundrone-ble-client -- interactive --lang zh
```

直接运行 release CLI：

```bash
./target/release/yundrone-ble-client interactive --lang zh
```

CLI 是显式子命令风格：

```bash
cargo run -p yundrone-ble-client -- --help
cargo run -p yundrone-ble-client -- interactive --help
cargo run -p yundrone-ble-client -- debug-ble --help
```

使用 `cargo run` 时，`--` 用来分隔 Cargo 参数和程序参数。直接运行 `./target/release/yundrone-ble-client` 时不需要这个分隔符。

## Debug 和开发

当设备搜不到、连接很卡、服务列表不出现，或者怀疑响应被截断时，先用 debug CLI：

```bash
cargo run -p yundrone-ble-client -- debug-ble \
  --target yundrone \
  --timeout 30 \
  --response-timeout 15 \
  --trace-chunks \
  --output /tmp/yundrone-ble-debug.log
```

开启 `--trace-chunks` 后，日志会显示：

- `[RX:raw]`：每条 BLE notify 原始 JSON 帧。
- `[RX:chunk]`：每个 `data.chunk` 分片，包含 `index/total`。
- `[RX:assembled]`：所有分片合并后的完整响应 JSON。
- `[OK] rx`：最终解码后的业务响应摘要。

常用本地检查命令：

```bash
scripts/ci/check.sh quality
```

这个脚本就是 GitHub Actions 里格式化、测试、clippy、release 脚本测试和版本一致性检查使用的同一个入口。Rust 版本由 [rust-toolchain.toml](./rust-toolchain.toml) 固定，因此本地检查和 CI 会使用同一套工具链。构建对齐命令也在同一个脚本里：`scripts/ci/check.sh build-full`、`scripts/ci/check.sh build-desktop`、`scripts/ci/check.sh package-macos`。

Release 版本由 [VERSION](./VERSION) 和 [CHANGELOG](./CHANGELOG) 管理。推送语义化 tag 后，release workflow 会发布 macOS app 资产。

## 文档导航

- Server 部署和 systemd 运维：[docs/systemd.md](./docs/systemd.md)
- BLE 调试器 JSON 指令指南：[docs/BLE_DEBUGGER_GUIDE_ZH.md](./docs/BLE_DEBUGGER_GUIDE_ZH.md)
- 协议命令与响应结构：[docs/COMMANDS.md](./docs/COMMANDS.md)
- 兼容 BLE server 实现说明：[docs/COMPATIBLE_BLE_SERVER_IMPLEMENTATION_ZH.md](./docs/COMPATIBLE_BLE_SERVER_IMPLEMENTATION_ZH.md)
- BLE MTU 分片中间件：[docs/MTU_CHUNKING_ZH.md](./docs/MTU_CHUNKING_ZH.md)
- 微信小程序搜索方案：[docs/WECHAT_MINIPROGRAM_DISCOVERY_ZH.md](./docs/WECHAT_MINIPROGRAM_DISCOVERY_ZH.md)
- Rust client library API：[docs/LIBRARY_API.md](./docs/LIBRARY_API.md)
- 新增命令开发指南：[docs/COMMAND_AUTHORING.md](./docs/COMMAND_AUTHORING.md)

## Release 和平台支持

GitHub Releases 当前只提供一个官方预编译资产：

- macOS Apple Silicon：`yundrone-ble-client-macos-arm64.zip`，内含 `yundrone-ble-client.app`。

平台状态：

- macOS：正式 tag release 会附带官方 GUI app。
- Linux：支持源码部署和 systemd 文档，但暂不附带官方预编译二进制。
- Windows：CI 会验证桌面构建，但暂不附带官方预编译二进制。

## 项目结构

本仓库是一个 Cargo workspace：

- `crates/protocol`：wire schema、typed request/response 和响应分片。
- `crates/server`：Linux BLE peripheral 和 NetworkManager 集成。Cargo package 与二进制名是 `yundrone-ble-server`。
- `crates/client`：BLE central library 和 CLI。Cargo package 与二进制名是 `yundrone-ble-client`。
- `crates/gui`：基于 `egui` 的原生桌面 GUI。
- `crates/platform_runtime`：平台启动辅助，主要服务 macOS app bundle 行为。

旧 Python 服务入口已经移除。当前部署路径使用 Rust server。

## 开源协议

MIT License.
