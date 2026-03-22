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

### 2. 运行命令行控制端 (CLI)
适合在服务器或纯终端环境下进行的交互式控制。
```bash
cargo run -p client -- --lang zh 
```

### 3. 构建设备端下位机程序 (Server)
在目标机器上编译提供广播与执行服务的常驻进程。
```bash
cargo build --release -p server
```
有关设置为随系统启停的后台服务，请参考 [docs/systemd.md](./docs/systemd.md)。

## 项目结构

本仓库使用 Cargo Workspace 管理，切分为以下四个子模块：

- `protocol/`: 核心数据协议层，包含分段算法与指令映射（无第三方依赖）。
- `server/`: 搭载在目标设备上的接收端（仅限 Linux 编译）。
- `client/`: 基于 `btleplug` 的蓝牙发送端 API 与命令行 TUI 工具。
- `gui/`: 基于 `egui` 构建的全平台图形交互客户端。

## 扩展与文档

- 若要为网关增加新的自定义指令，请参阅：[COMMAND_AUTHORING.md](./docs/COMMAND_AUTHORING.md)
- 项目开发计划：[TODO.md](./TODO.md)

## 开源协议

MIT License.
