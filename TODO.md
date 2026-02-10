# TODO

## Library-First (Current Milestone)

- [x] Expose high-level client API:
  - `BleGatewayClient.scan/scan_snapshot/connect`
  - `SessionHandle.run_command/provision/status/close`
- [x] Expose sync facade for GUI integration:
  - `SyncBleGatewayClient`
  - `SyncSessionHandle`
- [x] Keep CLI as a shell over library APIs (interactive flow remains).
- [x] Introduce unified library-facing models:
  - `DeviceInfo`, `CommandResult`, `ProvisionResult`, `StatusResult`
  - `GatewayError` + `GatewayErrorCode`
- [ ] Add more unit tests for library APIs:
  - disconnected session and reconnect semantics
  - sync facade lifecycle edge cases
  - command timeout / invalid argument coverage

## systemd Deployment (Server)

- [x] Provide a minimal unit template in-repo (`deploy/systemd/ble-command-gateway.service`).
- [x] Document manual install/start/inspect flow in `docs/systemd.md`.
- [ ] Add optional install/uninstall helper scripts later (not in current milestone).

## Future / Optional: Heartbeat (Not Scheduled)

Do not implement now. Re-open only if one of these triggers occurs:

- [ ] Long-lived session instability is observed in field logs (frequent stale-link operations).
- [ ] BLE stack shows silent disconnects without actionable signal from current command flow.
- [ ] GUI introduces persistent background sessions where explicit health state is required.

If triggered, heartbeat design must remain isolated from command protocol and queues.
