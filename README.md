# BLE Wi-Fi Provisioning

[![English](https://img.shields.io/badge/README-English-blue)](./README_EN.md)

通过 **跨平台客户端**（macOS / Linux / Windows）使用 BLE，为 **Linux 服务端** 下发 Wi-Fi 配置，并实时回传状态（`Connecting` / `Success_IP` / `Fail`）。

## 功能特性

- 客户端跨平台（`bleak` + `InquirerPy`）
- 服务端 Linux 专用（`bless` + BlueZ + `nmcli`）
- 客户端为常驻交互模式（菜单循环）
- 明确终态与退出码
- 支持 `systemd` 部署

## 项目结构

```text
.
├── client/
│   └── client_config_tool.py
├── server/
│   └── wifi_ble_service.py
├── config.py
├── sync_to_orin.sh
├── pyproject.toml
├── README.md
└── README_EN.md
```

## BLE 协议

- Service UUID: `A07498CA-AD5B-474E-940D-16F1FBE7E8CD`
- Write Characteristic（client -> server）: `51FF12BB-3ED8-46E5-B4F9-D64E2FEC021B`
- Read/Notify Characteristic（server -> client）: `51FF12BB-3ED8-46E5-B4F9-D64E2FEC021C`

写入 JSON：

```json
{"ssid": "LabWiFi", "pwd": "secret"}
```

## 环境要求

### 服务端（Linux）

- Ubuntu / Debian
- BlueZ
- NetworkManager（`nmcli`）
- Python `3.10-3.13`

安装系统依赖：

```bash
sudo apt update
sudo apt install -y network-manager bluez
```

### 客户端（跨平台）

- macOS / Linux / Windows
- 蓝牙硬件可用
- Python `3.10-3.13`

## 依赖管理（uv）

项目分组：

- `server`: `bless`, `dbus-next`
- `client`: `bleak`, `inquirerpy`
- 兼容性说明：服务端固定 `bless==0.3.0`，客户端 `bleak` 由 `uv` 自动解析（当前可解析到 2.x）。

安装：

```bash
# 服务端环境
uv sync --only-group server

# 客户端环境
uv sync --only-group client
```

安装完成后，直接使用 `.venv` 里的 Python 执行脚本（不需要 `uv run`）：

```bash
# 可选：激活虚拟环境
source .venv/bin/activate
```

## 快速开始

### 1) Linux 服务端启动

```bash
uv sync --only-group server
sudo -E "$(pwd)/.venv/bin/python" server/wifi_ble_service.py \
  --device-name Orin_Drone_01 \
  --ifname wlan0
```

### 2) 客户端启动交互菜单

```bash
uv sync --only-group client
"$(pwd)/.venv/bin/python" client/client_config_tool.py --target-name Orin_Drone_01
```

## 客户端交互菜单

启动后为常驻会话，可循环执行：

- 扫描并选择设备
- 修改设备名过滤条件
- 设置 Wi-Fi 凭据
- 执行配网（当前选中设备）
- 一键流程（扫描 -> 输入 -> 配网）
- 查看当前会话状态
- 退出

## HelloWorld 链路测试

`tests/helloworld/*.py` 现在是薄入口，核心实现在项目代码中：

- 服务端实现：`server/link_test_server.py`
- 客户端实现：`client/link_test_client.py`

运行方式：

```bash
# 1) Linux 端启动链路测试服务端
sudo -E "$(pwd)/.venv/bin/python" tests/helloworld/server_link_test.py --adapter hci0

# 2) 客户端执行链路测试（默认 10 次，支持 sequential/parallel）
"$(pwd)/.venv/bin/python" tests/helloworld/client_link_test.py \
  --target-name BLE_Hello_Server \
  --exchange-count 10 \
  --exchange-interval 1.0 \
  --exchange-mode sequential

# 推荐：更稳的链路测试参数（连接重试 + 较长连接超时）
"$(pwd)/.venv/bin/python" tests/helloworld/client_link_test.py \
  --target-name BLE_Hello_Server \
  --scan-timeout 30 \
  --connect-retries 6 \
  --connect-timeout 45 \
  --refresh-timeout 1.5 \
  --exchange-count 10 \
  --exchange-interval 1.0 \
  --exchange-mode sequential

# 仅在需要完整堆栈时再打开
# --full-traceback
```

如果服务端被中断后出现残留状态，可执行：

```bash
sudo -E "$(pwd)/.venv/bin/python" scripts/server_reset.py --adapter hci0
```

## 客户端退出码

- `0`: 成功
- `2`: 未发现设备
- `3`: 配网失败或 BLE 交互异常
- `4`: 等待终态超时
- `5`: 输入错误

## systemd 部署（服务端）

创建 `/etc/systemd/system/drone-ble.service`：

```ini
[Unit]
Description=Drone BLE Provisioning Service
After=bluetooth.target network.target

[Service]
Type=simple
WorkingDirectory=/home/nvidia/ble-wifi-provisioning
ExecStart=/home/nvidia/ble-wifi-provisioning/.venv/bin/python /home/nvidia/ble-wifi-provisioning/server/wifi_ble_service.py --device-name Orin_Drone_01 --ifname wlan0
User=root
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

启用：

```bash
cd /home/nvidia/ble-wifi-provisioning
uv sync --only-group server
sudo systemctl daemon-reload
sudo systemctl enable drone-ble.service
sudo systemctl start drone-ble.service
sudo systemctl status drone-ble.service
```

## 同步代码到远端

```bash
./sync_to_orin.sh
```

默认目标：`orin-Mocap5G:~/work/ble-wifi-provisioning/`

## 验证状态

已验证：

- `.venv/bin/ruff check .`
- `python3 -m py_compile config.py server/wifi_ble_service.py client/client_config_tool.py`

BLE 真机链路仍需在你的设备上做端到端测试。
