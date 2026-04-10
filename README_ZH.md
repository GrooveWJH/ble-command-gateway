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

## 快速上手

### 环境依赖
- Rust 开发环境 (`cargo`, `rustup`)
- 物理蓝牙适配器
- 服务端只支持 Linux 操作系统

### 1. 运行图形化配置端 (GUI)
跨平台启动带有配网、诊断和日志面板的用户界面。
```bash
cargo run -p gui
```
默认输入稳定前缀 `Yundrone_UAV`，扫描后再从候选列表里点选要连接的具体蓝牙实例。
在 macOS 上，GUI 会先通过共享运行时层自重启成签名 `.app`，再申请蓝牙权限。

### 2. 运行命令行控制端 (CLI)
适合在服务器或纯终端环境下进行的交互式控制。
```bash
cargo run -p client -- --lang zh 
```
CLI 会按前缀扫描所有匹配设备，展示完整广播名和 RSSI，并在出现多台候选设备时提示你手动选择目标设备。
在 macOS 上，CLI 也会经过同一套运行时兼容层后再访问 CoreBluetooth。

### 3. 构建设备端下位机程序 (Server)
在目标机器上编译提供广播与执行服务的常驻进程。
```bash
cargo build --release -p server
```
有关设置为随系统启停的后台服务，请参考 [docs/systemd.md](./docs/systemd.md)。

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
