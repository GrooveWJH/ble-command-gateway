# BLE Command Gateway (Rust)

[![English](https://img.shields.io/badge/README-English-blue)](./README_EN.md)

**云端无人机指控网关 (BLE Provisioning and Diagnostics Gateway)**：基于跨平台安全的 Rust 构建。本仓库提供了一个通过低功耗蓝牙 (BLE) 将 Wi-Fi 凭证下发到无头系统设备 (Linux / 树莓派 / Jetson Orin) 并提取网络和系统探针信息的完整开源解决方案。

原 `Python + bluedot / PySimpleGUI` 版本现已**100% 彻底以极致的模块化解耦标准用 Rust 重构**！全面支持了多国语言 (i18n)、超长 JSON 的跨端自动分片抓取、原生的高频异步并发线程池等。

---

## 🏗️ 全新 Crate 工作区架构 (Workspace)

项目被标准切割为了四个高度内聚的独立板块：

1. **`protocol` (核心协议)** (`crates/protocol`)
   - **零依赖**的纯算法核心。
   - `commands.rs`: 全局统一下发的指令键值对系统。
   - `chunking.rs`: 为抵抗蓝牙底层 MTU 收发限制（约 ~360 Bytes 最大），自研实现的自动封包拆解 / 组装引擎。

2. **`server` (Linux 外设服务端)** (`crates/server`)
   - *（仅限 Linux ARM/x86 编译）*
   - `main.rs`: 呼叫高底层 BlueZ D-Bus，作为 Peripheral 外设广播出 `Yundrone_UAV`。
   - `services.rs`: 使用 `tokio` 强力接管诸如 `nmcli device wifi connect`、`ifconfig` 和 `whoami` 等系统级指控。

3. **`client` (跨平台 CLI 控制台)** (`crates/client`)
   - `ble.rs`: 基于跨平台 `btleplug`，封装建立连接、UUID 通道定位与订阅的核心句柄。
   - `main.rs`: 指令级的 TUI。支持高颜值的终端表格 (`comfy-table`) 和隐藏式的安全密码下发输入 (`inquire`)，附带内建的简易 i18n 多语言翻译器。

4. **`gui` (原生解耦图形界面)** (`crates/gui`)
   - `main.rs`: 仅 **34 行** 的极简框架接驳点。
   - `i18n.rs`: 零依赖的多国语言词典 (中英无缝热切)。
   - `ble_worker.rs`: 在后台独立生长的守护 tokio 线程，屏蔽所有底层 IO 带来的桌面卡顿。
   - `app.rs`: 原生 `egui` 高刷画师，包含原汁原味的“核心配网”、“系统诊断”、“原始日志”三阶面板。

---

## 🚀 快速启动

你需要安装一套标准的 Rust 开发环境（`rustup`，包含 `cargo`）。

### 启动跨平台全干图形界面 (Mac / Win / Linux)
```bash
cargo run -p gui
```

### 启动安全沉浸式的命令行客户端
支持附加传入对应语言旗帜（默认 `zh`）。
```bash
cargo run -p client -- --lang en
```

### 编译下位机后台端程序 (在树莓派或 Jetson Linux 设备上执行)
```bash
cargo build --release -p server
# 生成的高性能二进制文件会安静地存放于 target/release/server
```

有关系统级自启运维 (systemd) 的教程请见 [docs/systemd.md](./docs/systemd.md)。

有关如何为设备加入新的自定义功能和回调请见 [docs/COMMAND_AUTHORING.md](./docs/COMMAND_AUTHORING.md)。

---

## 📝 许可证

这是一个供软硬结合边缘设备使用的控制协议方案，您可以在协议授权允许的范围内集成于实际产线的无人机设备、IoT 硬件出厂部署。
