# BLE 调试器调试指南

本文面向 Bluefruit Connect、nRF Connect、LightBlue 等通用 BLE 调试工具。你可以直接连接 YunDrone BLE Gateway，向 Write Characteristic 写入 JSON 指令，并在 Notify Characteristic 观察返回事件。

## 1. 先确认连的是对的设备

扫描页里可能同时出现系统展示名和 BLE 广播 `Local Name`。系统展示名可能是 `linux-board`，它不是业务身份。请进入设备详情，确认 `Local Name` 类似：

```text
yundrone-ytcwln
```

只要 `Local Name` 以当前前缀开头，例如默认 `yundrone-`，就可以连接。

如果 Bluefruit Connect 开启 `Must UART Service` 后设备消失，说明扫描阶段没有看到 Nordic UART Service UUID。这个过滤看的是 advertising data 或 scan response 中的 service UUID，不是连接后的 GATT 服务列表。当前 server 应声明：

```text
6e400001-b5a3-f393-e0a9-e50e24dcca9e
```

## 2. 连接后找这三个 UUID

| 类型 | UUID | 用途 |
| --- | --- | --- |
| Service | `6e400001-b5a3-f393-e0a9-e50e24dcca9e` | YunDrone 命令服务 |
| Write Characteristic | `6e400002-b5a3-f393-e0a9-e50e24dcca9e` | 写入请求 JSON |
| Notify Characteristic | `6e400003-b5a3-f393-e0a9-e50e24dcca9e` | 接收响应事件 |

操作顺序：

1. 连接设备。
2. 找到 Notify Characteristic 并开启 Notify / Subscribe。
3. 找到 Write Characteristic。
4. 用 UTF-8 / Text / String 模式写入 JSON。
5. 在 Notify Characteristic 里观察返回。

如果 Bluefruit Connect 卡在 `Discovering services`，说明它已经发起连接，但还没有拿到 GATT 服务列表。这个现象通常发生在服务端启动顺序不对：设备先开始广播，手机立刻连接，但服务端的 GATT service 还没注册完成。

当前 server 的正确启动顺序应该是：

```text
ble.gatt.ready
ble.advertising.ready
```

也就是先让 Nordic UART Service 和两个 characteristic 可发现，再开始广播。如果日志里看到 `ble.advertising.ready` 早于 `ble.gatt.ready`，请更新 server 或重启到新版服务。

现场处理步骤：

1. 在手机调试工具里断开当前连接。
2. 在手机系统蓝牙或调试工具里忘记/删除旧的缓存设备记录。
3. 在设备侧重启 `yundrone-ble-command-gateway.service`。
4. 重新扫描最新的 `yundrone-*` 实例。
5. 再连接并观察是否能进入 service 列表。

如果仍然卡住，请在设备侧同时看服务端日志和 BlueZ 日志。服务端日志要确认 `ble.gatt.ready` 已出现，BlueZ 日志里若反复出现连接后断开或 GATT cache 相关记录，就优先按缓存或链路问题处理。

如果你手边有本仓库源码，建议先用项目自带 debug CLI 排除服务端 GATT 表问题：

```bash
cargo run -p yundrone-ble-client -- debug-ble \
  --target yundrone --timeout 20 --response-timeout 10 \
  --trace-chunks --trace-qos \
  --output /tmp/yundrone-ble-debug.log
cat /tmp/yundrone-ble-debug.log
```

健康输出应至少包含：

```text
[OK] scan-match
[OK] connect
[OK] discover           ... 2 services
service 6e400001-b5a3-f393-e0a9-e50e24dcca9e primary=true <- UART service
char 6e400002-b5a3-f393-e0a9-e50e24dcca9e props=...
char 6e400003-b5a3-f393-e0a9-e50e24dcca9e props=...
[OK] subscribe
[QOS:ack]
[QOS:event-ack]
[OK] rx                 link.heartbeat ... text=alive
[OK] rx                 system.capabilities ... text=capabilities listed
```

如果 debug CLI 可以完整通过，而 Bluefruit 仍卡在 `Discovering services`，优先按 iOS / Bluefruit 缓存处理：

1. 在 Bluefruit 里断开并删除该设备记录。
2. 在 iOS 设置的蓝牙页面里忘记相关设备。
3. 关闭 Bluefruit，必要时关闭再打开手机蓝牙。
4. 重启设备侧 `yundrone-ble-command-gateway.service`。
5. 重新扫描最新的 `yundrone-*`。

我们在 Ubuntu/ARM 开发板上还遇到过两个容易混淆判断的系统层原因：

- 控制器默认地址是 `10:11:12:13:14:15` 这类假地址，多个设备或多次重装会被 iOS/macOS 当成同一个外设缓存。
- 如果强行把某些 combo Wi-Fi/Bluetooth 控制器改成 LE-only / static-addr，iOS/macOS 可能能建立 LE 连接，但服务发现前的 ATT MTU exchange 没有响应，表现就是 Bluefruit 长期卡在 `Discovering services`。

推荐服务端部署状态是“业务只用 BLE GATT，但控制器保持厂商默认 dual-mode”，并关闭系统配对入口：

```bash
sudo btmgmt info
bluetoothctl show | grep -E 'Name:|Alias:|Pairable:|UUID: Nordic|Roles'
```

期望看到：

```text
current settings: powered connectable br/edr le secure-conn
Pairable: no
UUID: Nordic UART Service
```

如果看到 `Pairable: yes`，请执行或安装仓库里的 `deploy/systemd/prepare-ble-adapter.sh`。它会关闭 pairable/bondable，但不会再强制 `bredr off` 或 `static-addr`。

## 3. 请求和响应格式

请求固定是：

```json
{"id":"debug-001","cmd":"link.heartbeat","args":{},"v":"YundroneBT-V2.1.0"}
```

响应是事件模型：

```json
{
  "id": "debug-001",
  "cmd": "link.heartbeat",
  "phase": "result",
  "seq": 1,
  "final": true,
  "ok": true,
  "code": "OK",
  "text": "alive",
  "data": {
    "alive": true
  },
  "v": "YundroneBT-V2.1.0"
}
```

关键字段：

- `id` 会原样返回，方便你把返回和请求对上。
- `cmd` 是服务端识别到的命令名。
- `phase` 为 `accepted`、`progress` 或 `result`。
- `seq` 是同一请求内递增序号。
- `final=true` 表示本次请求结束。

耗时命令会先回 `accepted`，执行中回 `progress`，最后回 `result`。调试工具里要等 `final=true` 才算真正结束。

## 4. 最小连通性测试：`link.heartbeat`

写入：

```json
{"id":"debug-heartbeat-001","cmd":"link.heartbeat","args":{},"v":"YundroneBT-V2.1.0"}
```

期望返回：

```json
{
  "id": "debug-heartbeat-001",
  "cmd": "link.heartbeat",
  "phase": "result",
  "seq": 1,
  "final": true,
  "ok": true,
  "code": "OK",
  "text": "alive",
  "data": {
    "alive": true
  },
  "v": "YundroneBT-V2.1.0"
}
```

如果没有返回，优先检查 Notify 是否已开启、写入位置是否是 Write Characteristic、写入模式是否是 UTF-8 文本。

## 5. 查看协议能力：`system.capabilities`

写入：

```json
{"id":"debug-cap-001","cmd":"system.capabilities","args":{},"v":"YundroneBT-V2.1.0"}
```

期望看到：

```json
{
  "id": "debug-cap-001",
  "cmd": "system.capabilities",
  "phase": "result",
  "seq": 1,
  "final": true,
  "ok": true,
  "code": "OK",
  "text": "capabilities listed",
  "data": {
    "protocol_version": "YundroneBT-V2.1.0",
    "commands": [
      "link.heartbeat",
      "system.status",
      "system.capabilities",
      "wifi.scan",
      "wifi.provision",
      "wifi.profiles.list",
      "wifi.profiles.delete"
    ],
    "features": [
      "response_events",
      "response_json_chunking",
      "wifi_profile_management"
    ],
    "payload_limit": 360
  },
  "v": "YundroneBT-V2.1.0"
}
```

## 6. 抓取系统状态：`system.status`

写入：

```json
{"id":"debug-status-001","cmd":"system.status","args":{},"v":"YundroneBT-V2.1.0"}
```

重点看返回里的：

- `data.device_name`: 对用户展示的公开 BLE 身份，例如 `yundrone-ytcwln`。
- `data.hostname`: 主机名。
- `data.system`: Linux 内核与架构。
- `data.user`: 应优先显示 SSH/部署用户，例如 `yundrone`。
- `data.network`: 当前连接的局域网名称。
- `data.ip`: 当前优先 IPv4。
- `data.interfaces`: 所有全局 IPv4 网卡。

调试器或正式 UI 应优先展示 `data.device_name`。`data.hostname` 只用于运维诊断，不应用作用户可见设备名。

## 7. 扫描 Wi-Fi：`wifi.scan`

写入：

```json
{"id":"debug-wifi-001","cmd":"wifi.scan","args":{},"v":"YundroneBT-V2.1.0"}
```

可选指定网卡：

```json
{"id":"debug-wifi-wlan0-001","cmd":"wifi.scan","args":{"ifname":"wlan0"},"v":"YundroneBT-V2.1.0"}
```

典型事件流：

```json
{"id":"debug-wifi-001","cmd":"wifi.scan","phase":"accepted","seq":1,"final":false,"ok":true,"code":"ACCEPTED","text":"accepted","v":"YundroneBT-V2.1.0"}
```

```json
{"id":"debug-wifi-001","cmd":"wifi.scan","phase":"progress","seq":2,"final":false,"ok":true,"code":"IN_PROGRESS","text":"please wait","v":"YundroneBT-V2.1.0"}
```

最后会收到 `final=true` 的结果，可能因为较大而被分片。完整结果重组后类似：

```json
{
  "id": "debug-wifi-001",
  "cmd": "wifi.scan",
  "phase": "result",
  "seq": 8,
  "final": true,
  "ok": true,
  "code": "OK",
  "text": "wifi scan complete",
  "data": {
    "ifname": null,
    "count": 2,
    "networks": [
      {
        "ssid": "ExampleWiFi",
        "channel": "6",
        "signal": 78
      }
    ]
  },
  "v": "YundroneBT-V2.1.0"
}
```

## 8. 下发配网：`wifi.provision`

开放网络：

```json
{"id":"debug-prov-open-001","cmd":"wifi.provision","args":{"ssid":"YourSSID"},"v":"YundroneBT-V2.1.0"}
```

有密码网络：

```json
{"id":"debug-prov-psk-001","cmd":"wifi.provision","args":{"ssid":"YourSSID","pwd":"example-password"},"v":"YundroneBT-V2.1.0"}
```

它也是耗时命令，会先返回 `accepted/progress`。最终成功类似：

```json
{
  "id": "debug-prov-psk-001",
  "cmd": "wifi.provision",
  "phase": "result",
  "final": true,
  "ok": true,
  "code": "PROVISION_SUCCESS",
  "text": "Provisioned Wi-Fi for YourSSID",
  "data": {
    "status": "connected",
    "ssid": "YourSSID",
    "ip": "192.0.2.x"
  },
  "v": "YundroneBT-V2.1.0"
}
```

不要在截图、日志或 issue 里泄露真实 Wi-Fi 密码。

## 9. 查看已保存 Wi-Fi：`wifi.profiles.list`

写入：

```json
{"id":"debug-profiles-list-001","cmd":"wifi.profiles.list","args":{},"v":"YundroneBT-V2.1.0"}
```

期望返回：

```json
{
  "id": "debug-profiles-list-001",
  "cmd": "wifi.profiles.list",
  "phase": "result",
  "final": true,
  "ok": true,
  "code": "OK",
  "text": "wifi profiles listed",
  "data": {
    "profiles": [
      {
        "uuid": "11111111-2222-3333-4444-555555555555",
        "name": "LabWiFi",
        "ssid": "LabWiFi",
        "active": true,
        "device": "wlan0",
        "autoconnect": true
      }
    ]
  },
  "v": "YundroneBT-V2.1.0"
}
```

删除时必须使用 `uuid`，不要用 SSID。因为 NetworkManager 里可能存在同名 SSID 或被改名的连接 profile。

## 10. 删除已保存 Wi-Fi：`wifi.profiles.delete`

默认安全删除非 active profile：

```json
{"id":"debug-profiles-delete-001","cmd":"wifi.profiles.delete","args":{"uuids":["11111111-2222-3333-4444-555555555555"]},"v":"YundroneBT-V2.1.0"}
```

如果选中了当前 active profile，且没有 `force=true`，服务端会跳过：

```json
{
  "id": "debug-profiles-delete-001",
  "cmd": "wifi.profiles.delete",
  "phase": "result",
  "final": true,
  "ok": true,
  "code": "PROTECTED_PROFILE",
  "text": "wifi profiles delete finished: deleted=0, skipped=1, failed=0",
  "data": {
    "deleted": [],
    "skipped": [
      {
        "uuid": "11111111-2222-3333-4444-555555555555",
        "name": "LabWiFi",
        "ssid": "LabWiFi",
        "reason": "active_profile"
      }
    ],
    "failed": []
  },
  "v": "YundroneBT-V2.1.0"
}
```

调试器可以手写 `force:true`，但这可能删除当前联网配置并让设备掉线。GUI 默认不会暴露这个能力。

## 11. 大响应分片怎么看

如果某个最终响应超过协议单帧预算 `360 bytes`，你会看到多条带 `data.chunk` 的 Notify：

```json
{
  "id": "debug-wifi-001",
  "cmd": "wifi.scan",
  "phase": "result",
  "seq": 8,
  "final": true,
  "ok": true,
  "code": "OK",
  "text": "",
  "data": {
    "chunk": {
      "mode": "response_json",
      "index": 1,
      "total": 4,
      "payload": "{\"id\":\"debug-wifi-001\",..."
    }
  },
  "v": "YundroneBT-V2.1.0"
}
```

手工重组方法：

1. 按相同 `id` 分组。
2. 按 `data.chunk.index` 从小到大排序。
3. 拼接每片 `data.chunk.payload`。
4. 拼出来的字符串就是完整 response JSON。

GUI / CLI 会自动重组，调试工具通常不会。

## 12. 错误请求示例

旧命令会被拒绝：

```json
{"id":"debug-old-ping","cmd":"ping","args":{},"v":"YundroneBT-V2.1.0"}
```

期望错误：

```json
{
  "id": "debug-old-ping",
  "cmd": "ping",
  "phase": "result",
  "final": true,
  "ok": false,
  "code": "UNKNOWN_COMMAND",
  "text": "unknown command: ping",
  "v": "YundroneBT-V2.1.0"
}
```

旧协议版本也会被拒绝：

```json
{"id":"debug-old-version","cmd":"link.heartbeat","args":{},"v":"YundroneBT-V1.0.0"}
```

期望 `code=BAD_REQUEST`，`text` 包含 `unsupported protocol version`。

## 13. 同时看服务端日志

在目标机上打开：

```bash
sudo journalctl -u yundrone-ble-command-gateway.service -f -o cat
```

正常请求链路应看到：

```text
ble.gatt.ready service_uuid=6e400001-b5a3-f393-e0a9-e50e24dcca9e
ble.advertising.ready identity_name=yundrone-...
ble.request.received request_id=debug-heartbeat-001 cmd=link.heartbeat
ble.response.sent request_id=debug-heartbeat-001 cmd=link.heartbeat response_code=OK phase=Result chunk_count=1
```

如果是 `wifi.scan` 这类大响应，日志应显示 `accepted/progress/result` 事件，以及最终结果是否进入 `chunked response_json`。

## 14. 快速检查清单

1. Local Name 是否以 `yundrone-` 开头。
2. 是否能看到 Nordic UART Service UUID。
3. Bluefruit 是否已经通过 `Discovering services`，进入 service 列表。
4. 服务端日志里 `ble.gatt.ready` 是否早于 `ble.advertising.ready`。
5. Notify 是否已开启。
6. JSON 是否写进了 Write Characteristic。
7. 写入格式是否是 UTF-8 文本。
8. 请求是否包含 `id/cmd/args/v`。
9. `v` 是否是 `YundroneBT-V2.1.0`。
10. 服务端日志是否出现 `ble.request.received` 或 `ble.request.parse_failed`。
11. 大响应是否只是被分片，而不是没返回。

最小可用测试永远是：

```json
{"id":"debug-heartbeat-001","cmd":"link.heartbeat","args":{},"v":"YundroneBT-V2.1.0"}
```
