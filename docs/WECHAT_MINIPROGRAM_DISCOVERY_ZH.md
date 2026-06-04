# 微信小程序 BLE 搜索方案

本文说明微信小程序应如何发现 YunDrone BLE Gateway。目标是让小程序稳定找到设备，同时避免被系统蓝牙名、缓存名或调试工具的展示名误导。

## 先看现象

在 Bluefruit Connect 这类调试工具里，同一个设备可能同时出现两个名字：

- 列表标题：`linux-board`
- 详情字段：`Local Name: yundrone-lab1-k9x8`

小程序不应该把列表标题当作业务身份。列表标题通常来自系统缓存、BlueZ adapter 的 `Name/Alias`，或连接后的 GATT 设备名。它可能显示为主机名，例如 `linux-board`。

我们真正设计并控制的是广播数据里的 Local Name，也就是新安装默认的 `yundrone-<alias4>-<random4>`，例如 `yundrone-lab1-k9x8`。小程序发现设备时必须以这个字段为准。旧设备可能仍显示 `yundrone-<base36_6>`，也应继续兼容。

Linux server 启动后还会把 BlueZ adapter `Alias` 同步为同一个 `yundrone-*` 公开身份，并用 pretty hostname / `/etc/bluetooth/main.conf` 的 `yundrone` 作为启动兜底。这样可以降低调试工具列表标题显示硬件主机名的概率，但小程序的业务识别仍应以 `localName` 为准。

## 可用字段

微信小程序扫描回调里的设备对象通常包含这些字段：

| 字段 | 建议用途 | 说明 |
| --- | --- | --- |
| `localName` | 主要发现身份 | 广播数据里的 LocalName 字段，应匹配 `yundrone-*` |
| `name` | 只做兜底显示 | 可能来自系统缓存、广播名或连接后的 GATT name，不稳定 |
| `deviceId` | 连接句柄 | iOS/Android 含义不同，不要作为长期设备身份 |
| `RSSI` | 排序和信号提示 | 数值越接近 0 信号越强 |
| `advertisServiceUUIDs` | 扫描过滤信号 | server 会声明 Nordic UART service UUID，便于 Bluefruit / 微信按服务过滤 |
| `advertisData` | 高级兜底 | ManufacturerData，不作为当前默认识别路径 |

当前项目的发现原则固定为：

- 发现阶段只把 `localName` 以 `yundrone-` 开头的设备加入候选列表。
- 不要用 `name === "linux-board"` 或类似系统名做判断。
- 如果使用 `services` 过滤，过滤值应为 Nordic UART service UUID；但候选展示仍必须以 `localName` 为准，避免把其他 UART 设备当成 YunDrone。
- 连接成功后必须检查 UART service UUID 和读写 characteristic UUID，防止误连。

## 推荐扫描流程

1. 调用 `wx.openBluetoothAdapter()` 打开蓝牙模块。
2. 先注册 `wx.onBluetoothDeviceFound()`，再启动扫描，避免漏掉早期回调。
3. 调用 `wx.startBluetoothDevicesDiscovery()`。
4. 扫描期间实时处理 `res.devices`，把命中 `localName.startsWith("yundrone-")` 的设备追加到候选列表。
5. 候选出现后立即允许用户点击连接，不必等扫描窗口结束。
6. 用户点击候选后立刻调用 `wx.stopBluetoothDevicesDiscovery()`。
7. 调用 `wx.createBLEConnection({ deviceId })` 建立连接。
8. 连接后调用 `wx.getBLEDeviceServices()`，确认存在 `6e400001-b5a3-f393-e0a9-e50e24dcca9e`。
9. 再调用 `wx.getBLEDeviceCharacteristics()`，确认写入 UUID 和通知 UUID 存在。
10. 验证通过后才进入配网、状态查询、Wi-Fi 扫描等业务页面。

## 推荐扫描参数

首次扫描有两种可选策略。产品默认建议先不传 `services`，用 `localName` 做候选判断；如果你希望系统先帮你过滤 UART 设备，可以传 Nordic UART service UUID：

```js
wx.startBluetoothDevicesDiscovery({
  allowDuplicatesKey: true,
  interval: 0,
  powerLevel: 'high',
})
```

服务过滤版本：

```js
wx.startBluetoothDevicesDiscovery({
  services: ['6e400001-b5a3-f393-e0a9-e50e24dcca9e'],
  allowDuplicatesKey: true,
  interval: 0,
  powerLevel: 'high',
})
```

参数含义：

- `allowDuplicatesKey: true` 允许同一设备重复上报，便于更新 RSSI 和补齐迟到的 `localName`。
- `interval: 0` 表示发现后尽快上报。它不是扫描总时长。
- `powerLevel: 'high'` 可提升 Android 微信上的扫描积极性；不支持的平台会忽略或报兼容问题，实际项目里应做兼容处理。
- 不传 `services` 时，候选面更宽，适合排障和兼容系统缓存字段不完整的场景。
- 传 `services` 时，候选更干净，但仍要在回调里检查 `localName.startsWith("yundrone-")`。

扫描总时长由业务层自己控制，例如 30 秒：

```js
const SCAN_TIMEOUT_MS = 30000

const scanTimer = setTimeout(() => {
  wx.stopBluetoothDevicesDiscovery()
  setScanning(false)
}, SCAN_TIMEOUT_MS)
```

## 候选去重策略

候选列表应按 `deviceId` 去重，但展示身份应使用 `localName`。

原因是 `deviceId` 是小程序连接 API 所需的句柄，而 `localName` 才是用户能理解的设备身份。

```js
const TARGET_PREFIX = 'yundrone-'
const candidates = new Map()

function handleFoundDevice(device) {
  const localName = device.localName || ''

  if (!localName.startsWith(TARGET_PREFIX)) {
    return
  }

  const previous = candidates.get(device.deviceId)

  candidates.set(device.deviceId, {
    deviceId: device.deviceId,
    localName,
    rssi: typeof device.RSSI === 'number' ? device.RSSI : previous?.rssi,
    lastSeenAt: Date.now(),
  })

  renderCandidates([...candidates.values()])
}

wx.onBluetoothDeviceFound((res) => {
  for (const device of res.devices || []) {
    handleFoundDevice(device)
  }
})
```

如果产品想给用户看一个更宽松的调试列表，可以显示所有有名字的设备，但候选连接列表仍只展示 `localName` 命中 `yundrone-` 的设备。

## 点击候选后连接

点击候选后不要继续扫描。扫描会消耗系统资源，也可能影响连接稳定性。

```js
const UART_SERVICE_UUID = '6e400001-b5a3-f393-e0a9-e50e24dcca9e'
const WRITE_UUID = '6e400002-b5a3-f393-e0a9-e50e24dcca9e'
const NOTIFY_UUID = '6e400003-b5a3-f393-e0a9-e50e24dcca9e'

async function connectCandidate(candidate) {
  wx.stopBluetoothDevicesDiscovery()
  setConnecting(candidate.deviceId)

  await wx.createBLEConnection({ deviceId: candidate.deviceId })

  const servicesResult = await wx.getBLEDeviceServices({
    deviceId: candidate.deviceId,
  })

  const uartService = servicesResult.services.find((service) => {
    return service.uuid.toLowerCase() === UART_SERVICE_UUID
  })

  if (!uartService) {
    await wx.closeBLEConnection({ deviceId: candidate.deviceId })
    throw new Error('这不是 YunDrone BLE Gateway：未发现 UART service')
  }

  const charsResult = await wx.getBLEDeviceCharacteristics({
    deviceId: candidate.deviceId,
    serviceId: uartService.uuid,
  })

  const hasWrite = charsResult.characteristics.some((ch) => {
    return ch.uuid.toLowerCase() === WRITE_UUID && ch.properties.write
  })

  const hasNotify = charsResult.characteristics.some((ch) => {
    return ch.uuid.toLowerCase() === NOTIFY_UUID && ch.properties.notify
  })

  if (!hasWrite || !hasNotify) {
    await wx.closeBLEConnection({ deviceId: candidate.deviceId })
    throw new Error('这不是 YunDrone BLE Gateway：UART characteristics 不完整')
  }

  setConnected({
    deviceId: candidate.deviceId,
    localName: candidate.localName,
    serviceId: uartService.uuid,
  })
}
```

## UI 建议

扫描页建议展示三类状态：

- 扫描中：显示“正在搜索 yundrone-* 设备”，并显示停止扫描按钮。
- 已发现候选：候选一出现就可点击，不等 30 秒。
- 未发现：扫描超时后提示“没有发现 yundrone-*，请确认设备已开机并处于 10 分钟快刀广播窗口内”。

候选项建议展示：

```text
yundrone-lab1-k9x8
RSSI -42 dBm · 刚刚发现
```

不要把 `name` 字段作为主标题。若为了排障必须展示，可以放在折叠的调试信息里：

```text
系统展示名: linux-board
Local Name: yundrone-lab1-k9x8
deviceId: ...
```

## 常见坑

### 1. 为什么 Bluefruit 顶部显示 `linux-board`？

那通常不是当前项目的业务广播名，而是系统蓝牙适配器名、缓存名或连接后 GATT name。小程序不要依赖它。

### 2. 扫描时能不能传 `services`？

可以。server 会在广播元数据里声明 Nordic UART service UUID，所以 Bluefruit Connect 的 `Must UART Service` 或微信的 `services` 过滤可以识别它。

但 `services` 只能说明“它像一个 Nordic UART 设备”，不能说明“它一定是 YunDrone”。最终候选仍看 `localName`，连接后仍要验证 GATT service 和 characteristic。

### 3. `deviceId` 能不能保存起来下次直连？

不建议。`deviceId` 是平台连接句柄，iOS 与 Android 语义不同，也可能随系统策略变化。更稳的方式是每次按 `localName` 重新发现，用户选择后再连接。

### 4. 如果多个设备都叫 `yundrone-*` 怎么办？

这是正常的。列表按 RSSI 排序，并显示完整 `localName`。当前命名里的 4 位别名方便用户识别设备，4 位随机码用于区分同名别名设备，例如 `yundrone-lab1-k9x8`。旧设备上的 6 位后缀也仍然可显示和连接。

### 5. 连接后还要验证服务吗？

必须验证。扫描阶段只负责发现候选，连接后的 GATT service / characteristic 才是最终身份校验。

## 最终口径

小程序搜索 YunDrone 设备时，使用下面这条链路：

```text
localName startsWith "yundrone-"
  -> 加入候选列表
  -> 用户点击候选
  -> stopBluetoothDevicesDiscovery
  -> createBLEConnection
  -> getBLEDeviceServices 验证 UART service
  -> getBLEDeviceCharacteristics 验证 write/notify
  -> 进入业务命令通道
```

这条链路把“发现用的名字”和“连接后服务验证”分开处理，能避开 `linux-board` 这类系统展示名带来的误判。
