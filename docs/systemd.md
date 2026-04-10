# systemd 部署指南

本项目提供了一个 systemd 单元服务模板，用于在边缘设备（如 Jetson Orin / 树莓派）上部署 Rust 服务端。

- 模板位置：`deploy/systemd/ble-command-gateway.service`

## 1) 安装构建依赖

在 Orin / Ubuntu 设备上至少需要这些系统依赖，否则 `bluer` 的 `libdbus-sys` 会编译失败：

```bash
sudo apt update
sudo apt install -y pkg-config libdbus-1-dev
```

如需在设备上本机编译，还需要可用的 Rust 工具链。若尚未安装，可执行：

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
```

## 2) 构建服务端二进制

```bash
. "$HOME/.cargo/env"
cd /opt/ble-command-gateway
cargo build --release -p server
```

## 3) 调整与安装 systemd 单元

确认单元文件使用 Rust 二进制入口：

```ini
[Unit]
Description=BLE Command Gateway Server
After=bluetooth.service NetworkManager.service
Wants=bluetooth.service NetworkManager.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/ble-command-gateway
Environment="SUDO_USER=root"
ExecStart=/opt/ble-command-gateway/target/release/server
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

安装命令：

```bash
sudo cp deploy/systemd/ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ble-command-gateway.service
```

## 4) 查看状态与日志

```bash
sudo systemctl status ble-command-gateway.service --no-pager
sudo journalctl -u ble-command-gateway.service -f
```

服务启动后，日志中会打印结构化关键事件。部署验证时，至少应看到下面这些关键字：

- `ble.server.starting`
  需要同时带出 `adapter_name` 与 `advertised_name`
- `ble.advertising.ready`
  说明广播已经起来，可看到实际广播名
- `ble.gatt.ready`
  说明 GATT 服务、读写 UUID 都已就绪
- `ble.request.received`
  需要同时带出 `request_id`、`cmd`
- `ble.response.sent`
  需要同时带出 `request_id`、`cmd`、`response_code`、`chunk_count`

部署验证时请记录日志里的 `advertised_name`，例如 `Yundrone_UAV-15-19-A7F2`，然后在 CLI / GUI 中按前缀 `Yundrone_UAV` 扫描，再从候选列表里选择对应实例。

建议直接用下面的命令过滤关键日志：

```bash
sudo journalctl -u ble-command-gateway.service -f | rg 'ble\\.(server|advertising|gatt|request|response)'
```

若要核对某次请求的完整链路，可按 `request_id` 过滤：

```bash
sudo journalctl -u ble-command-gateway.service --since "10 min ago" | rg 'request_id='
```

若启动失败，请优先检查：

```bash
sudo journalctl -u ble-command-gateway.service -n 100 --no-pager
```

常见阻塞项：
- 缺少 `libdbus-1-dev` / `pkg-config`，导致 `cargo build` 失败
- 蓝牙适配器未开启或 `bluetooth.service` 未运行
- 设备侧未安装 `NetworkManager` / `nmcli`

## 5) BlueZ 广告 interval 排障提示

在部分 Linux 设备上，server 日志虽然会打印快刀广告 interval，例如 `20 ms`，但控制器最终可能仍以默认慢 interval 广播。

我们在 `OrangePi 4 Pro` + BlueZ `5.64` 上实测过一种情况：如果 `bluetoothd` 未带 `--experimental` 启动，BlueZ 会看到 D-Bus 广告对象中的 `MinInterval` / `MaxInterval`，但不会把它们继续下发到 mgmt/HCI。

这不是当前已知会在所有 Linux 设备上必现的问题，但如果你看到“应用日志说自己在快刀广播，实机却很难被扫描到”，建议按下面步骤核对。

### 建议排查步骤

1. 先抓真实 HCI 参数，而不是只看应用日志

```bash
sudo btmon
```

重点关注 `LE Set Extended Advertising Parameters`，确认 interval 是否真的落成你期望的值。

2. 如果你怀疑设备受这个问题影响，可给 `bluetooth.service` 增加 override：

```ini
[Service]
ExecStart=
ExecStart=/usr/lib/bluetooth/bluetoothd --experimental
```

3. 重新加载并重启蓝牙与服务：

```bash
sudo systemctl daemon-reload
sudo systemctl restart bluetooth.service
sudo systemctl restart ble-command-gateway.service
```

4. 再次用 `btmon` 验证 interval 是否真正变化

如果你的板卡本来就能正确应用 `MinInterval` / `MaxInterval`，则不需要为此调整系统配置。

## 6) 禁用与卸载

```bash
sudo systemctl disable --now ble-command-gateway.service
sudo rm -f /etc/systemd/system/ble-command-gateway.service
sudo systemctl daemon-reload
```
