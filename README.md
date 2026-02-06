# BLE Command Gateway

[![English](https://img.shields.io/badge/README-English-blue)](./README_EN.md)

BLE 配网与设备诊断网关：通过 BLE 将 Wi‑Fi 指令从客户端下发到 Linux 设备端，并返回可观测状态与系统信息。

- English README: `README_EN.md`
- 路线图与待办: `TODO.md`

## 概述

本项目面向设备首次联网、现场换网以及无屏诊断场景，提供一套可脚本化、可观测、可扩展的 BLE 指令通道与交互式客户端。

## 主要特性

- 交互式客户端：扫描、选设备、长连接会话复用、Rich UI 输出
- 配网能力：下发 SSID/密码（支持开放网络），服务端通过 `nmcli` 执行并回传终态与 IP
- 诊断指令：`status / sys.whoami / net.ifconfig / wifi.scan` 等用于现场定位问题
- 可观测性：服务端配网阶段日志与进行中状态回传；客户端等待与分片接收可视化
- 可扩展性：协议编解码、命令注册、系统执行、BLE 网关分层清晰，便于持续演进

## 架构

```text
app/        入口（server_main.py / client_main.py）
ble/        BLE 网关、运行时封装、响应发布（含分片）
protocol/   协议结构、编解码、状态码
commands/   命令注册与内置命令实现
services/   系统命令执行、Wi‑Fi 配网服务
client/     扫描、连接会话、交互流程、渲染
config/     默认配置与 UUID
tests/      单元/集成测试
```

## BLE 协议

- Service UUID: `6E400001-B5A3-F393-E0A9-E50E24DCCA9E`
- Client Write Char: `6E400002-B5A3-F393-E0A9-E50E24DCCA9E`
- Server Read/Notify Char: `6E400003-B5A3-F393-E0A9-E50E24DCCA9E`

请求示例（JSON）：

```json
{"id":"req-1","cmd":"provision","args":{"ssid":"LabWiFi","pwd":"secret"}}
```

## 命令

- `help`
- `ping`
- `status`
- `provision`
- `shutdown`
- `sys.whoami`
- `net.ifconfig`
- `wifi.scan`

## 环境要求

- Python：`3.10 - 3.13`
- 依赖管理：`uv`
- 服务端：仅支持 Linux（BlueZ + NetworkManager `nmcli`）

系统依赖（Debian/Ubuntu）：

```bash
sudo apt update
sudo apt install -y network-manager bluez
```

Python 依赖：

```bash
uv sync --only-group server
uv sync --only-group client
```

## 快速开始

服务端（Linux）：

```bash
sudo -E "$(pwd)/.venv/bin/python" app/server_main.py \
  --device-name Yundrone_UAV \
  --ifname wlan0 \
  --adapter hci0 \
  --log-level INFO
```

客户端（macOS/Linux/Windows）：

```bash
"$(pwd)/.venv/bin/python" app/client_main.py --target-name Yundrone_UAV
```

## 建议流程

- 最小联通性验证：`help` -> `status` -> `wifi.scan` -> `provision`
- `status`：校验当前 SSID 与 IP，确认换网是否生效
- `wifi.scan`：确认目标 SSID 可见且信号强度可接受

## 测试

```bash
python3 -m py_compile app/server_main.py app/client_main.py
python3 -m unittest discover -s tests/unit -p 'test_*.py'
```

## 部署

- 推荐以 `app/server_main.py` 作为 systemd `ExecStart`
- 服务端通常以 `sudo` 启动；`status/whoami` 会优先返回实际操作者账号（如 `SUDO_USER`）

示例：

```ini
ExecStart=/path/to/.venv/bin/python /path/to/app/server_main.py --device-name Yundrone_UAV --ifname wlan0 --adapter hci0
```

## 路线图

- 链路心跳与断联判定（见 `TODO.md`）
- 更细粒度的配网进度事件模型
- 更完整的端到端自动化与压力测试
