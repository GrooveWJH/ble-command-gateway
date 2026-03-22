# systemd 部署指南

本项目提供了一个极简的 systemd 单元服务模板，用于在边缘设备（如 Jetson Orin / 树莓派）上部署服务端。

- 模板位置：`deploy/systemd/ble-command-gateway.service` (需手动创建)

## 1) 调整路径与运行参数

在安装前，请创建单元文件并填入正确的 Rust 编译产物路径：

```ini
[Unit]
Description=BLE Command Gateway Server
After=network.target bluetooth.target

[Service]
Type=simple
# 请修改为您实际编译出的 Rust server 二进制文件路径
ExecStart=/opt/ble-command-gateway/target/release/server
# 确保 nmcli 等命令能正确获取系统级别的网络接口权限
Environment="SUDO_USER=root"
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## 2) 安装服务

```bash
sudo cp deploy/systemd/ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ble-command-gateway
```

## 3) 查看状态与日志

```bash
sudo systemctl status ble-command-gateway
sudo journalctl -u ble-command-gateway -f
```

## 4) 禁用与卸载

```bash
sudo systemctl disable --now ble-command-gateway
sudo rm -f /etc/systemd/system/ble-command-gateway.service
sudo systemctl daemon-reload
```
