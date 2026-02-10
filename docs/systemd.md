# systemd Deployment (Minimal)

This project provides a minimal unit template at:

- `deploy/systemd/ble-command-gateway.service`

## 1) Adjust paths and runtime args

Before installing, update these fields in the unit file:

- `WorkingDirectory`
- `ExecStart` (venv path, device name, interface, adapter, log level)

## 2) Install unit

```bash
sudo cp deploy/systemd/ble-command-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable ble-command-gateway
sudo systemctl restart ble-command-gateway
```

## 3) Check status and logs

```bash
sudo systemctl status ble-command-gateway
sudo journalctl -u ble-command-gateway -f
```

## 4) Disable/remove

```bash
sudo systemctl disable --now ble-command-gateway
sudo rm -f /etc/systemd/system/ble-command-gateway.service
sudo systemctl daemon-reload
```
