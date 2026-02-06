# TODO

## Heartbeat (Client <-> Server)

- [ ] Add a dedicated low-level heartbeat channel that does not interfere with command traffic.
- [ ] Add heartbeat GATT characteristics (separate from command read/write): client sends heartbeat, server replies ack.
- [ ] Heartbeat cadence: client sends every 1s.
- [ ] Timeout rule: single heartbeat >3s without ack counts as timeout.
- [ ] Disconnect rule: 5 consecutive heartbeat timeouts => mark link disconnected.
- [ ] Keep heartbeat fully isolated from command protocol parsing/queues.
- [ ] Add server-side lightweight heartbeat logs (recv/ack/last seen).
- [ ] Add client-side session health monitor and reconnect hint on disconnect.
- [ ] Refactor interactive client runtime to support true background heartbeat task (async session loop).
- [ ] Wire menu connection lamp to heartbeat session status (green=alive, red=disconnected).

## systemd Deployment (Server)

- [ ] Define a systemd unit for `app/server_main.py` (e.g. `ble-command-gateway.service`).
- [ ] Decide unit location and ownership:
  - Prefer installing to `/etc/systemd/system/` for production (package-less deployment).
  - Keep the unit template in-repo under `deploy/systemd/` (or `docs/systemd/`) for versioning.
- [ ] Startup strategy:
  - Start on boot with a delay (e.g. `ExecStartPre=/bin/sleep 30`) to avoid early-boot BLE/DBus instability.
  - Ensure NetworkManager and Bluetooth are ready: `After=bluetooth.service NetworkManager.service` and `Wants=` accordingly.
- [ ] Runtime requirements:
  - Run as root (or via capabilities) because BlueZ advertising + nmcli typically need elevated privileges.
  - Set working directory and venv python path explicitly.
  - Configure `--device-name`, `--ifname`, `--adapter`, `--log-level`.
- [ ] Logging:
  - Use journald; ensure logs include device name and timestamps.
  - Document how to inspect: `journalctl -u ble-command-gateway -f`.
- [ ] Provide an install/uninstall helper script (non-interactive):
  - Install: copy unit file, `systemctl daemon-reload`, `enable`, `restart`.
  - Uninstall: `disable`, stop, remove unit, reload.
  - Decide script location: `scripts/install_systemd.sh` (and `scripts/uninstall_systemd.sh`) or `tools/systemd/`.
