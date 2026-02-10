# Library API Overview

Primary entrypoints:

- `client.BleGatewayClient` (async)
- `client.SyncBleGatewayClient` (sync facade for GUI/non-async callers)

Contract:

- Library APIs do not write directly to stdout/stderr.
- Progress/status text is emitted only via optional `reporter` callbacks.

## Async usage

```python
from client import BleGatewayClient

gateway = BleGatewayClient(target_name="Yundrone_UAV")
devices = await gateway.scan(timeout=8)
session = await gateway.connect(devices[0])
status = await session.status(timeout=8)
print(status.message)
await session.close()
```

## Sync usage

```python
from client import SyncBleGatewayClient

gateway = SyncBleGatewayClient(target_name="Yundrone_UAV")
devices = gateway.scan(timeout=8)
session = gateway.connect(devices[0])
status = session.status(timeout=8)
print(status.message)
session.close()
gateway.close()
```

## Models

- `DeviceInfo`
- `ScanSnapshot`
- `CommandResult`
- `ProvisionResult`
- `StatusResult`
- `GatewayError`, `GatewayErrorCode`
