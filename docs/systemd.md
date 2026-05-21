# systemd 部署指南

本项目提供了一个 systemd 单元服务模板，用于在边缘设备（如 Jetson Orin / 树莓派）上部署 Rust 服务端。

- 模板位置：`deploy/systemd/yundrone-ble-command-gateway.service`

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
rustup toolchain install 1.95.0 --profile minimal --component rustfmt --component clippy
```

## 2) 构建或安装服务端二进制

当前现场 release 部署使用：

```text
/opt/yundrone/ble-command-gateway/current/yundrone-ble-server
```

如果目标机从源码构建，也可以继续使用仓库模板里的源码路径：

```bash
. "$HOME/.cargo/env"
cd /opt/ble-command-gateway
cargo build --release -p yundrone-ble-server
```

## 3) 调整与安装 systemd 单元

确认单元文件使用 Rust 二进制入口：

```ini
[Unit]
Description=YunDrone BLE Command Gateway Server
After=bluetooth.service NetworkManager.service
Wants=bluetooth.service NetworkManager.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/ble-command-gateway
Environment=YUNDRONE_BLE_ADV_BACKEND=bluez-dbus
Environment=YUNDRONE_LOG_COLOR=always
Environment=YUNDRONE_DEVICE_PREFIX=yundrone
ExecStartPre=/opt/ble-command-gateway/deploy/systemd/prepare-ble-adapter.sh
ExecStartPre=-/usr/bin/bluetoothctl pairable off
ExecStart=/opt/ble-command-gateway/target/release/yundrone-ble-server
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

建议同时为系统显示名和 BlueZ 设置不会暴露硬件型号的默认名。server 启动后会把 Adapter `Alias` 动态同步为完整公开身份，例如 `yundrone-ytcwln`；下面配置用于 bluetoothd 刚启动、server 尚未接管前的兜底，并且不修改 Linux static hostname：

```bash
sudo hostnamectl set-hostname --pretty yundrone
```

`/etc/bluetooth/main.conf`:

```ini
[General]
Name = yundrone
```

部署脚本 `deploy/systemd/prepare-ble-adapter.sh` 会自动写入这个兜底项，并在服务启动前关闭系统配对入口。

不要在部分 ARM 板载 combo Wi-Fi/Bluetooth 控制器上强制 `ControllerMode = le`、`btmgmt bredr off` 或 `btmgmt static-addr`。实测这可能导致 iOS/macOS 可以建立 LE 连接，但 ATT MTU exchange 没有响应，Bluefruit 会卡在 `Discovering services`。当前推荐保持控制器默认 dual-mode，只关闭 pairable/bondable，业务层仍然只使用 BLE GATT。

可用下面命令检查当前控制器显示面：

```bash
hostnamectl status
bluetoothctl show | grep -E 'Name:|Alias:'
```

建议同时安装 `deploy/systemd/prepare-ble-adapter.sh` 并保持可执行。该脚本会在 server 启动前完成 BLE 公开身份和控制器策略收敛：

- 写入 `Name = yundrone`，作为 bluetoothd 启动早期的公开名称兜底
- 保持控制器默认 dual-mode，不强制 `ControllerMode = le`，避免部分控制器在 iOS/macOS 服务发现阶段卡住
- 保持 BR/EDR 打开；业务层仍然只暴露 BLE GATT，不依赖经典蓝牙 profile
- 关闭 bondable / pairable，当前 UART JSON 链路不需要系统配对
- 不设置 static random address；设备身份由持久化 BLE local name 与 GATT 服务共同确认

安装命令：

```bash
sudo chmod +x deploy/systemd/prepare-ble-adapter.sh
sudo cp deploy/systemd/yundrone-ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now yundrone-ble-command-gateway.service
```

## 4) 查看状态与日志

```bash
sudo systemctl status yundrone-ble-command-gateway.service --no-pager
sudo journalctl -u yundrone-ble-command-gateway.service -f -o cat
```

普通 `journalctl` 会额外显示 systemd 自己的时间与进程前缀。服务端现在会输出适合人读的彩色多行摘要，因此排障时推荐使用 `-o cat` 保留应用日志原貌。

服务启动后，日志中会打印结构化关键事件。部署验证时，至少应看到下面这些关键字：

- `ble.server.starting`
  需要同时带出 `adapter_name` 与 `identity_name`
- `ble.advertising.ready`
  说明广播已经起来，可看到实际广播名
- `ble.gatt.ready`
  说明 GATT 服务、读写 UUID 都已就绪
- `ble.request.received`
  需要同时带出 `request_id`、`cmd`
- `ble.response.sent`
  需要同时带出 `request_id`、`cmd`、`response_code`、`chunk_count`、`chunk_mode`、`payload_limit`、`response_bytes`、`max_chunk_bytes`

还应检查适配器策略：

```bash
sudo btmgmt info
bluetoothctl show | grep -E 'Name:|Alias:|Pairable:|UUID: Nordic|Roles'
```

推荐状态：

```text
current settings: powered connectable bondable br/edr le secure-conn
Pairable: no
UUID: Nordic UART Service
```

如果 `Pairable` 仍是 `yes`，手机系统或调试工具可能会弹系统配对请求。请确认 systemd 中的 `prepare-ble-adapter.sh` 已成功执行。不要为了消除 `br/edr` 字段而执行 `btmgmt bredr off`；在部分 ARM 板载 combo 控制器上，这会让 Bluefruit 等 iOS/macOS 工具卡在服务发现阶段。

响应日志还会附带一段多行摘要，便于直接判断是否触发 transport frame：

```text
BLE response
  request: wifi.scan / 0692fc09-3533-4a63-8112-a2e62a0f2db9
  result: OK ok=true
  transport: transport
  bytes: response=1842 limit=20 max_chunk=20
  chunks: count=116 sizes=[20, 20, ...]
```

日志不会打印完整响应 payload，也不会打印 Wi-Fi 密码；传输部分只展示大小、数量和传输模式。V2 主路径中，最终结果会被拆成 20B 以内的 transport frame；周期性“仍在进行”提示是 header-only `Progress` 控制帧，不会显示成大 JSON 分片。

部署验证时请记录日志里的 `identity_name`，例如 `identity_name=yundrone-ytcwln`，然后在 CLI / GUI 中按前缀 `yundrone` 扫描，再从候选列表里选择对应实例。

设备名持久化在 `/var/lib/yundrone/ble-device-name`。这个文件是 BLE local name 的权威来源，不放在 `/opt/yundrone/ble-command-gateway` 或 `/opt/ble-command-gateway`，避免代码部署覆盖设备身份。文件缺失或内容非法时，server 会重建为 `<prefix>-<base36_6>`；如果无法写入该文件，server 应启动失败，避免产生临时名字污染移动端缓存。

建议直接用下面的命令过滤关键日志：

```bash
sudo journalctl -u yundrone-ble-command-gateway.service -f -o cat | rg 'ble\\.(server|advertising|gatt|request|response)|BLE (server|advertising|response)'
```

若要核对某次请求的完整链路，可按 `request_id` 过滤：

```bash
sudo journalctl -u yundrone-ble-command-gateway.service --since "10 min ago" -o cat | rg 'request_id=|request:'
```

若启动失败，请优先检查：

```bash
sudo journalctl -u yundrone-ble-command-gateway.service -n 120 -o cat --no-pager
```

常见阻塞项：
- 缺少 `libdbus-1-dev` / `pkg-config`，导致 `cargo build` 失败
- 蓝牙适配器未开启或 `bluetooth.service` 未运行
- 设备侧未安装 `NetworkManager` / `nmcli`

## 5) BlueZ 广告 interval 排障提示

在部分 Linux 设备上，server 日志虽然会打印快刀广告 interval，例如 `20 ms`，但控制器最终可能仍以默认慢 interval 广播。

我们在某些 Ubuntu/ARM 开发板 + BlueZ `5.64` 上实测过一种情况：如果 `bluetoothd` 未带 `--experimental` 启动，BlueZ 会看到 D-Bus 广告对象中的 `MinInterval` / `MaxInterval`，但不会把它们继续下发到 mgmt/HCI。

这已经是发现优先部署配置中的重点核对项。如果你看到“应用日志说自己在快刀广播，实机却很难被扫描到”，建议按下面步骤核对。

### 建议排查步骤

1. 先抓真实 HCI 参数，而不是只看应用日志

```bash
sudo btmon
```

重点关注 `LE Set Extended Advertising Parameters`，确认 interval 是否真的落成你期望的值。
当前发现优先策略应在快刀阶段接近 `20 ms`，并在 `600 s` 后切换到 `211.25 ms` 稳态。

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
sudo systemctl restart yundrone-ble-command-gateway.service
```

4. 再次用 `btmon` 验证 interval 是否真正变化
5. 同时确认 advertising local name 是唯一身份，如 `yundrone-ytcwln`
6. 面向微信小程序等移动端 client 时，把单一 `yundrone-*` local name 作为发现身份，把连接后的 GATT 服务发现作为最终验证链路

如果你的板卡本来就能正确应用 `MinInterval` / `MaxInterval`，则不需要为此调整系统配置。

## 6) 禁用与卸载

```bash
sudo systemctl disable --now yundrone-ble-command-gateway.service
sudo rm -f /etc/systemd/system/yundrone-ble-command-gateway.service
sudo systemctl disable --now ble-command-gateway.service 2>/dev/null || true
sudo rm -f /etc/systemd/system/ble-command-gateway.service
sudo systemctl daemon-reload
```
