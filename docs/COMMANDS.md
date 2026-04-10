# Command Contracts

This document defines the stable `CommandResponse.data` payloads returned by the BLE gateway.

All responses share the top-level schema from `protocol::CommandResponse`:

```json
{
  "id": "request-id",
  "ok": true,
  "code": "OK",
  "text": "human readable summary",
  "data": {},
  "v": "YundroneBT-V1.0.0"
}
```

## Commands

### `help`

- `code`: `OK`
- `text`: summary string
- `data.commands`: array of supported command names

### `ping`

- `code`: `OK`
- `text`: `pong`
- `data.pong`: boolean

### `status`

- `code`: `OK` or command failure code
- `text`: status summary
- `data.hostname`: hostname string
- `data.system`: `uname -srm` string
- `data.user`: effective user string

### `sys.whoami`

- `code`: `OK` or command failure code
- `text`: effective user string
- `data.user`: effective user string

### `net.ifconfig`

- `code`: `OK`, `INTERNAL_ERROR`, or `TIMEOUT`
- `text`: raw `ifconfig` output
- `data`: omitted

### `wifi.scan`

- `code`: `OK`, `INTERNAL_ERROR`, or `TIMEOUT`
- `text`: scan summary
- `data.ifname`: interface name string
- `data.count`: number of networks
- `data.networks`: array of network objects

Each network object contains:

- `ssid`: string
- `channel`: string
- `signal`: integer

### `provision`

- success:
  - `code`: `PROVISION_SUCCESS`
  - `text`: provisioning summary
  - `data.status`: `connected`
  - `data.ssid`: target SSID
  - `data.ip`: resolved IP string or `Unknown IP`
- failure:
  - `code`: `PROVISION_FAIL` or `BAD_REQUEST`
  - `text`: failure summary
  - `data.status`: `failed` when command execution started but failed
  - `data.ssid`: target SSID when known

### `shutdown`

- `code`: `OK`, `INTERNAL_ERROR`, or `TIMEOUT`
- `text`: command result text
- `data`: omitted
