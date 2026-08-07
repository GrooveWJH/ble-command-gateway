# WebBluetooth 配网工作台

本文说明仓库内 `web-client/` 的定位、运行方式、配网流程和限制。

> 状态：可源码运行和静态构建，但尚未纳入正式安装器、release asset 或公网发布链路。

公网试用入口：

```text
https://tool.yundrone.cn/ble/
```

## 它是什么

`web-client/` 是一个 Carbon Design System 风格的高密度静态 WebBluetooth 配网工作台，用浏览器直接连接附近的 YunDrone BLE 设备。它主要填补“用户无法或不方便安装 CLI / GUI，但需要给面前已安装 server 的设备配 Wi-Fi”的场景。

构建产物是纯静态 HTML/CSS/JS。公网部署只需要托管 `web-client/dist/`，不需要额外后端服务、API 服务或部署守护进程。

它面向当前 `main` 分支 server 的 JSON UART 协议：

- 请求：`id/cmd/args/v`
- 响应：`id/cmd/phase/seq/final/ok/code/text/data/v`
- 大响应：`data.chunk.mode = "response_json"`
- QoS：请求 accepted 阶段会用同一 request id 重试；收到可靠 chunk 后发送 `link.ack` chunk ACK，完整 response event 交付后发送 `link.ack` event ACK

它不使用 20B compact binary transport；那套方案已归档到独立实验分支，后续仅作为微信小程序 BLE 4.0 兼容方向继续研究。

## 用户配网流程

发布成 HTTPS 页面后，非 CLI 用户可以按下面流程完成配网：

1. 用桌面 Chrome / Edge 或 Android Chrome 打开页面。
2. 确认目标设备已上电，并让设备靠近当前电脑或手机。
3. 点击“连接设备”，在浏览器蓝牙选择器里选择安装器提示过的 `yundrone-` 开头设备，例如 `yundrone-12abcd`。
4. 连接成功后页面会自动进入“基本信息”，读取 `system.status` 与 `system.capabilities`。
5. 切到“Wi-Fi 配网”，点击“扫描 Wi-Fi”，等待 10 到 30 秒。
6. 在扫描结果里选择目标 SSID，或直接手动输入隐藏网络 SSID。
7. 输入 Wi-Fi 密码；开放网络可以留空。密码只用于本次下发，不保存到 `localStorage`。
8. 点击“确认并下发配网”，确认设备名和 SSID 后等待结果。
9. 成功后页面会自动刷新系统状态并回到“基本信息”，优先展示最新 IP；失败时页面会提示检查密码、频段兼容性和信号。

页面一次只执行一个业务命令，避免 BLE 弱链路下并发命令互相干扰。

界面是固定视口工作台：顶部是连接区，中间是标签页工作区，底部是默认收起的 Debug 状态栏。页面本身不做上下滚动；内容较多时只在当前 panel 内部滚动。“基本信息”优先展示设备名、主机名、系统、当前网络、首选 IP、接口列表、协议版本、payload limit、命令和特性。“Wi-Fi 配网”页内嵌紧凑步骤提示，并用高密度表格展示 Wi-Fi 扫描结果。

## 调试模式

Debug 抽屉对应 CLI verbose 的人类可读版本：

- `Safe Verbose`：默认模式，会显示 TX/RX、chunk、assembled response、ACK、elapsed 等信息，并脱敏 `pwd/password/psk` 等字段。
- `Unsafe Raw`：显示未脱敏原始内容，只适合本地安全环境排查。
- `Off`：隐藏大部分协议细节，只保留必要命令状态。

默认只显示底部状态栏，避免挤压主工作区。展开 Debug 抽屉后支持过滤、暂停、清空和导出 `.trace` 文本，便于用户把失败现场发给开发者。

默认使用 `Safe Verbose`。只有在本地可信环境排查问题时才建议切到 `Unsafe Raw`，因为它可能显示未脱敏的原始命令内容。

## 本地运行

```bash
cd web-client
pnpm install
pnpm dev
```

然后用支持 WebBluetooth 的浏览器打开 Vite 输出的本地地址，通常是：

```text
http://localhost:5173
```

`localhost` 属于浏览器允许 WebBluetooth 的安全上下文。

## 构建和验证

```bash
cd web-client
pnpm test
pnpm build
pnpm check:maxlines
```

构建产物会生成到 `web-client/dist/`。该目录是本地构建产物，不应提交。

`check:maxlines` 默认要求 `ts/tsx/scss/md` 文件不超过 250 行，用来避免前端页面重新堆成一个难维护的大文件。

## 浏览器限制

WebBluetooth 连接发生在访问者自己的浏览器和他身边的 BLE 设备之间，服务器不会代替用户连接蓝牙。

当前建议环境：

- 桌面 Chrome / Edge
- Android Chrome
- `localhost` 或 HTTPS 页面

不建议或不可用：

- Safari / Firefox
- iPhone / iPad 上的普通浏览器
- 公网 HTTP 页面

## 公网试用

如果只是临时试用，可以把 `pnpm build` 生成的 `dist/` 当静态网站部署到 HTTPS 平台，例如 Cloudflare Pages、Vercel、GitHub Pages 或自有 Nginx。

当前自有 Nginx 部署路径固定为 `https://tool.yundrone.cn/ble/`。Vite 构建的 `base` 必须保持为 `/ble/`，否则静态资源在子路径下会加载失败。

当前不修改正式部署文档和安装脚本，因为 Web client 尚未被认定为完成版，也还没有进入 release 分发流程。

如果页面被嵌入 iframe，外层站点还需要允许浏览器蓝牙权限，例如设置合适的 `Permissions-Policy: bluetooth=(self)`。第一版建议直接用独立页面发布，先不要嵌 iframe。

## 常见失败提示

- 页面提示不支持 Web Bluetooth：换桌面 Chrome / Edge 或 Android Chrome，并确认是 HTTPS / localhost。
- 浏览器蓝牙选择器里看不到设备：确认设备上电、BLE server 正在运行、设备名以 `yundrone-` 开头，并靠近浏览器设备。
- 扫描 Wi-Fi 超时或失败：设备可能离路由器太远，或 NetworkManager/无线网卡不可用。
- 配网失败：优先检查密码、SSID 是否选错，以及目标设备是否支持该 Wi-Fi 频段。
- 调试信息里多次 request retry：说明请求 accepted 阶段不稳定，通常是蓝牙距离、系统负载或 server 状态问题。

## 和现有客户端的关系

当前正式客户端仍是：

- `crates/client`：CLI 和 BLE central library
- `crates/gui`：原生桌面 GUI

`web-client/` 是新增的静态网页入口，用于探索“无需安装本地客户端、直接用支持 WebBluetooth 的浏览器完成配网”的工作流。它不属于 Cargo workspace，也不会改变 Rust server/client/protocol 的 wire schema。
