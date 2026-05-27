# BLE Command Contracts V2

This document defines the wire commands and typed response data for protocol `YundroneBT-V2.1.0`.

V2 is a breaking protocol. Legacy commands such as `ping`, `help`, `status`, `sys.whoami`, `net.ifconfig`, `provision`, and `shutdown` are not formal commands anymore.

## Request Schema

```json
{
  "id": "request-id",
  "cmd": "domain.action",
  "args": {},
  "v": "YundroneBT-V2.1.0"
}
```

- `id`: caller-generated request ID. Every response event for the request reuses it.
- `cmd`: V2 command name.
- `args`: command-specific object. Use `{}` when the command has no arguments.
- `v`: protocol version. The current server expects `YundroneBT-V2.1.0`.

## Response Event Schema

```json
{
  "id": "request-id",
  "cmd": "wifi.scan",
  "phase": "result",
  "seq": 1,
  "final": true,
  "ok": true,
  "code": "OK",
  "text": "human readable summary",
  "data": {},
  "v": "YundroneBT-V2.1.0"
}
```

- `cmd`: original command name when available.
- `phase`: `accepted`, `progress`, or `result`.
- `seq`: monotonically increasing event number within the same request.
- `final`: `true` only for the terminal event.
- `ok/code/text/data`: command result fields.

Fast commands usually return one `result` event. Slow foreground commands return:

1. `accepted`
2. one or more `progress` events, usually once per second
3. one final `result`

The response chunking middleware can split any oversized event into multiple BLE notifications. GUI/CLI clients reassemble this transparently.

V2.1 adds lightweight transport acknowledgements. Clients ACK every reliable chunk and every completed response event with `link.ack`; applications should not expose `link.ack` as a user-facing command.

## Commands

### `link.heartbeat`

Purpose: application-level liveness check for GUI heartbeat and debugger smoke tests.

Arguments: none.

Response:

- `code`: `OK`
- `data.alive`: boolean

### `link.ack`

Purpose: transport-level acknowledgement for response chunks and completed response events.

Arguments:

- `ack_type`: `chunk` or `event`
- `response_seq`: response event sequence number being acknowledged
- `chunk_index`: required for `ack_type=chunk`, omitted for `ack_type=event`

Response: none. The server consumes this command in the transport layer and does not emit a business response.

### `system.status`

Purpose: combined system and network diagnostics.

Arguments: none.

Response:

- `code`: `OK`, `INTERNAL_ERROR`, or `TIMEOUT`
- `data.device_name`: public BLE identity string, for example `yundrone-ytcwln`
- `data.hostname`: hostname string
- `data.system`: `uname -srm` string
- `data.user`: preferred operator user string
- `data.network`: active LAN/Wi-Fi connection name when available
- `data.ip`: preferred IPv4 address when available
- `data.interfaces[]`: `{ ifname, kind, ipv4 }`

`kind` is `wifi`, `ethernet`, or `other`.

User-facing apps should display `data.device_name` as the device identity. `data.hostname` is diagnostic only and may reveal the Linux host name.

### `system.capabilities`

Purpose: discover server protocol and feature support.

Arguments: none.

Response:

- `code`: `OK`
- `data.protocol_version`: protocol version string
- `data.commands[]`: supported V2 command names
- `data.features[]`: feature flags
- `data.payload_limit`: protocol single-frame budget in bytes

### `wifi.scan`

Purpose: scan nearby Wi-Fi APs through NetworkManager.

Arguments:

- `ifname`: optional Wi-Fi interface name, for example `wlan0`

Event behavior: slow command with `accepted/progress/result`.

Final response:

- `code`: `OK`, `INTERNAL_ERROR`, or `TIMEOUT`
- `data.ifname`: requested interface or `null`
- `data.count`: number of returned network entries
- `data.networks[]`: `{ ssid, channel, signal }`

### `wifi.provision`

Purpose: save credentials and connect to a Wi-Fi network.

Arguments:

- `ssid`: required target SSID
- `pwd`: optional password. Omit for open networks.

Event behavior: slow command with `accepted/progress/result`.

Final success:

- `code`: `PROVISION_SUCCESS`
- `data.status`: `connected`
- `data.ssid`: target SSID
- `data.ip`: resolved IP when available

Final failure:

- `code`: `PROVISION_FAIL`, `BAD_REQUEST`, or `TIMEOUT`
- `data.status`: `failed`
- `data.ssid`: target SSID

### `wifi.profiles.list`

Purpose: list saved NetworkManager Wi-Fi connection profiles.

Arguments: none.

Response:

- `code`: `OK`, `INTERNAL_ERROR`, or `TIMEOUT`
- `data.profiles[]`: `{ uuid, name, ssid, active, device, autoconnect }`

Use `uuid` as the stable delete key. Do not delete by SSID because duplicate SSIDs and renamed NetworkManager profiles are common.

### `wifi.profiles.delete`

Purpose: delete saved NetworkManager Wi-Fi profiles by UUID.

Arguments:

- `uuids`: required string array of profile UUIDs
- `force`: optional boolean, default `false`

Event behavior: slow command with `accepted/progress/result`.

Safety:

- Active profiles are skipped when `force=false`.
- GUI does not expose `force=true`.
- Manual debugger use of `force=true` can disconnect the device from the current network.

Final response:

- `code`: `OK` when every requested deletion succeeded
- `code`: `PARTIAL_SUCCESS` when some profiles were deleted but some were skipped or failed
- `code`: `PROTECTED_PROFILE` when only active protected profiles were selected
- `data.deleted[]`: `{ uuid, name, ssid }`
- `data.skipped[]`: `{ uuid, name, ssid, reason }`
- `data.failed[]`: `{ uuid, name, ssid, error }`

The active-profile skip reason is `active_profile`.
