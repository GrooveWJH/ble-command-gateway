# BLE Wi-Fi Provisioning

[![English](https://img.shields.io/badge/README-English-blue)](./README_EN.md)

通过 **跨平台客户端**（macOS / Linux / Windows）使用 BLE，为 **Linux 服务端** 下发 Wi-Fi 配置，并实时回传状态（`Connecting` / `Success_IP` / `Fail`）。

## 当前架构（重构后）

```text
.
├── app/                    # 应用入口
│   ├── server_main.py
│   └── client_main.py
├── ble/                    # BLE 网关/运行时/发布器
├── protocol/               # 协议 envelope / code / command id
├── commands/               # 指令注册、加载、内置指令
├── services/               # 配网服务、系统命令服务
├── client/                 # 交互流程与命令调用
├── config/                 # UUID 与默认参数
├── server/                 # 仅保留 preflight、link_test 等专项模块
├── scripts/                # legacy 启动器
├── tools/                  # 运维/legacy 实际脚本
└── tests/                  # unit/integration/e2e
```

## BLE 协议

- Service UUID: `6E400001-B5A3-F393-E0A9-E50E24DCCA9E`
- RX Characteristic（client -> server write）: `6E400002-B5A3-F393-E0A9-E50E24DCCA9E`
- TX Characteristic（server -> client read/notify）: `6E400003-B5A3-F393-E0A9-E50E24DCCA9E`

请求 JSON 示例：

```json
{"id":"req-1","cmd":"provision","args":{"ssid":"LabWiFi","pwd":"secret"}}
```

## 内置命令

- `help`
- `ping`
- `status`
- `provision`
- `shutdown`
- `sys.whoami`
- `net.ifconfig`

提示：`help` 默认返回命令列表；详细用法请使用 `help` + `args.cmd`。

## 环境要求

### 服务端（Linux）

- Ubuntu / Debian
- BlueZ
- NetworkManager（`nmcli`）
- Python `3.10-3.13`

```bash
sudo apt update
sudo apt install -y network-manager bluez
```

### 客户端（跨平台）

- macOS / Linux / Windows
- 蓝牙硬件可用
- Python `3.10-3.13`

## 依赖管理（uv）

```bash
# 服务端
uv sync --only-group server

# 客户端
uv sync --only-group client
```

## 快速开始

### 1) 启动服务端（新入口）

```bash
uv sync --only-group server
sudo -E "$(pwd)/.venv/bin/python" app/server_main.py \
  --device-name Orin_Drone_01 \
  --ifname wlan0 \
  --adapter hci0
```

### 2) 启动客户端（新入口）

```bash
uv sync --only-group client
"$(pwd)/.venv/bin/python" app/client_main.py --target-name Orin_Drone_01
```

## 手机直连调试

可用 LightBlue / nRF Connect 直接写 RX 特征：

```json
{"id":"req-help-1","cmd":"help","args":{}}
```

如需详细命令说明：

```json
{"id":"req-help-2","cmd":"help","args":{"cmd":"provision"}}
```

## 链路测试

```bash
# 服务端
sudo -E "$(pwd)/.venv/bin/python" tests/integration/server_link_test.py --adapter hci0

# 客户端
"$(pwd)/.venv/bin/python" tests/integration/client_link_test.py \
  --target-name BLE_Hello_Server \
  --exchange-count 10 \
  --exchange-mode sequential
```

## 运维脚本

- 重置蓝牙状态：

```bash
sudo -E "$(pwd)/.venv/bin/python" tools/reset/server_reset.py --adapter hci0
```

- `scripts/bless_uart.py` 为 legacy demo 启动器，默认拒绝执行；仅在显式 `--run-legacy` 时运行。

## systemd 部署

`ExecStart` 请使用新入口：

```ini
ExecStart=/home/nvidia/ble-wifi-provisioning/.venv/bin/python /home/nvidia/ble-wifi-provisioning/app/server_main.py --device-name Orin_Drone_01 --ifname wlan0
```

## 验证

```bash
python3 -m py_compile app/server_main.py app/client_main.py ble/server_gateway.py
python3 -m unittest discover -s tests/unit -p 'test_*.py'
```

BLE 真机链路仍需在你的设备上做端到端验证。
