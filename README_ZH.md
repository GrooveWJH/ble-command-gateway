# YunDrone BLE Gateway

[![English](https://img.shields.io/badge/README-English-blue?style=flat-square)](./README.md)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?style=flat-square&logo=rust)](#)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](#)

YunDrone BLE Gateway 是一个基于低功耗蓝牙 (BLE) 的无头设备通讯网关，旨在为缺少网络和显示器的边缘 Linux 设备（如树莓派、Jetson）提供 Wi-Fi 配网与系统诊断能力。

本项目完全采用 Rust 编写，支持跨平台运行，能够在不依赖云端网络的情况下完成物理穿透控制。

## 核心特性

- **协议分片 (Chunking)**：内置自定义分段重组算法，突破底层蓝牙硬件的 MTU 负载限制，可稳定传输千字节级大型 JSON。
- **服务端 (Server)**：专为 Linux 平台优化的外设守护进程，结合 `bluer` 与 `nmcli` 实现网络配置与系统命令执行。
- **客户端 (Client/GUI)**：提供终端命令行 (TUI) 与图形化视窗 (`egui`) 两种形态的跨平台控制端。
- **多设备可区分广播名**：服务端启动时会生成形如 `Yundrone_UAV-HH-MM-ABCD` 的动态蓝牙名，避免多台设备同时开机时出现难以区分的同名前缀实例。
- **并发与安全**：基于 Rust 与 `tokio` 构建，实现蓝牙底层 I/O 与前端渲染的隔离。

## 安装与部署

这个项目分成两种角色：

- `server`：跑在目标 Linux 设备上，负责 BLE 广播、接收指令、执行配网与诊断
- `client` / `gui`：跑在你的电脑上，负责扫描并连接到 `server`

推荐顺序是：

1. 先安装 Rust 工具链
2. 再部署 Linux `server`
3. 最后在本机运行 `client` 或 `gui`

### 1. 安装 Rust 工具链

#### macOS

先安装 Apple 命令行开发工具：

```bash
xcode-select --install
```

再安装 Rust：

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

#### Ubuntu / Debian

先安装本机构建依赖：

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libdbus-1-dev libudev-dev
```

再安装 Rust：

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

#### Windows

先安装带 C++ 工具链的 Visual Studio Build Tools，再安装 Rust：

```powershell
winget install Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc
rustup component add rustfmt clippy
rustc --version
cargo --version
```

### 2. 拉取仓库

```bash
git clone https://github.com/GrooveWJH/ble-command-gateway.git
cd ble-command-gateway
```

### 3. 部署 Linux 服务端

`server` crate 只支持 Linux，适合部署在需要被配网和被诊断的无头设备上。

#### 3.1 在目标设备安装运行依赖

目标设备至少需要：

- BlueZ / `bluetoothd`
- `NetworkManager` 与 `nmcli`
- 如果要本机编译，还需要 `pkg-config` 和 `libdbus-1-dev`

Ubuntu 系设备可参考：

```bash
sudo apt update
sudo apt install -y bluetooth bluez network-manager pkg-config libdbus-1-dev
```

#### 3.2 构建服务端

如果是在目标设备本机编译：

```bash
source "$HOME/.cargo/env"
cargo build --release -p server
```

构建产物路径为：

```text
target/release/server
```

#### 3.3 在挂 systemd 之前先手动启动一次

```bash
sudo ./target/release/server
```

若启动正常，日志中至少应看到：

- `ble.server.starting`
- `ble.advertising.ready`
- `ble.gatt.ready`

日志里还会打印当前真实广播名，例如：

```text
Yundrone_UAV-15-19-A7F2
```

#### 3.4 安装 systemd 服务

仓库里已经带了一个可直接调整的 unit 文件：

```text
deploy/systemd/ble-command-gateway.service
```

常见安装流程：

```bash
sudo cp deploy/systemd/ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ble-command-gateway.service
```

查看状态与日志：

```bash
sudo systemctl status ble-command-gateway.service --no-pager
sudo journalctl -u ble-command-gateway.service -f
```

完整部署说明请看 [docs/systemd.md](./docs/systemd.md)。

### 4. 安装并运行客户端

客户端有两个入口：

- `gui`：桌面图形配置端
- `client`：终端交互式 CLI

如需一起构建：

```bash
cargo build --release -p gui -p client
```

#### 4.1 运行 GUI

开发态运行：

```bash
cargo run -p gui
```

发布态运行：

```bash
./target/release/gui
```

打包成可在 Finder 中双击启动的 macOS `.app`：

```bash
chmod +x scripts/package-macos-gui.sh
./scripts/package-macos-gui.sh
open "target/release/YunDrone BLE Gateway.app"
```

这个命令会先构建 release 版 GUI，再生成正式的 `.app` 包目录，拷贝项目里的 `Info.plist`，并做一次 ad-hoc 签名，让 Finder 可以把它当成普通 macOS 应用启动。

使用流程：

1. 输入稳定前缀 `Yundrone_UAV`
2. 点击扫描
3. 从候选列表中点选具体蓝牙实例
4. 连接成功后再进入配网与诊断面板

在 macOS 上，GUI 会先经由共享运行时层重启成签名 `.app`，再申请蓝牙权限。

#### 4.2 运行 CLI

开发态运行：

```bash
cargo run -p client -- --lang zh
```

发布态运行：

```bash
./target/release/client --lang zh
```

CLI 会按前缀扫描所有匹配的 BLE 实例，显示完整广播名与 RSSI，并要求你手动选择具体设备后再连接。

在 macOS 上，CLI 也会先经过同一套运行时兼容层后再访问 CoreBluetooth。

### 5. 部署后的推荐验收

当 `server` 和 `client/gui` 都安装完之后，建议按下面顺序验收：

1. 启动 Linux `server`
2. 打开 `gui` 或 `client`
3. 使用前缀 `Yundrone_UAV` 扫描
4. 确认能看到完整广播实例名
5. 连接后至少执行一次：
   - Wi-Fi 扫描
   - status
   - ping
6. 确认服务端日志能看到对应的 `request_id` 与响应日志

## 项目结构

本仓库使用 Cargo Workspace 管理，切分为以下五个子模块：

- `protocol/`: 核心数据协议层，包含分段算法与指令映射（无第三方依赖）。
- `platform_runtime/`: 统一的平台运行时兼容层，负责 macOS app bundle 启动准备以及 Linux/Windows 空实现。
- `server/`: 搭载在目标设备上的接收端（仅限 Linux 编译）。
- `client/`: 基于 `btleplug` 的蓝牙发送端 API 与命令行 TUI 工具。
- `gui/`: 基于 `egui` 构建的全平台图形交互客户端。

## 扩展与文档

- 若要为网关增加新的自定义指令，请参阅：[COMMAND_AUTHORING.md](./docs/COMMAND_AUTHORING.md)
- 命令返回结构说明：[COMMANDS.md](./docs/COMMANDS.md)
- Rust 客户端库接口说明：[LIBRARY_API.md](./docs/LIBRARY_API.md)
- 项目开发计划：[TODO.md](./TODO.md)

仓库中的旧 Python 服务入口已移除；当前部署路径不再依赖 Python 运行时代码。

## 排障提示

### Linux 服务端：某些 BlueZ 环境下广告 interval 可能不会真正生效

在某些 Linux 设备上，服务端日志虽然会打印快刀广告间隔，例如 `20 ms`，但蓝牙控制器最终仍可能以更慢的默认间隔广播。

我们在以下环境中实际观测到过这个问题：

- `OrangePi 4 Pro`
- BlueZ `5.64`
- `bluetoothd` 未带 `--experimental` 启动

在这套环境里，BlueZ 会在 D-Bus 广告对象上看到 `MinInterval` / `MaxInterval`，但在继续下发到 mgmt/HCI 前静默忽略它们。给 `bluetoothd` 加上：

```ini
ExecStart=/usr/lib/bluetooth/bluetoothd --experimental
```

之后，这台机器上的广告 interval 已经能够正确下发到 mgmt/HCI。

请注意：

- 这里记录的是“可能出现的平台相关问题”，不是所有 Linux 机器都会必现。
- 如果你看到服务端日志声称自己在 `20 ms` 广播，但设备依然很难被扫描到，请优先抓 `btmon` 看真实 HCI 参数，而不要只看应用日志。

针对受影响机器的推荐应对方案：

1. 为 `bluetooth.service` 添加 systemd override，让 `bluetoothd` 以 `--experimental` 启动
2. 重启 `bluetooth.service`
3. 重启 `ble-command-gateway.service`
4. 用 `btmon` 验证 `LE Set Extended Advertising Parameters` 是否已变为目标间隔

示例 override：

```ini
[Service]
ExecStart=
ExecStart=/usr/lib/bluetooth/bluetoothd --experimental
```

## 开源协议

MIT License.
