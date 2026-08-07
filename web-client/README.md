# YunDrone WebBluetooth 配网工作台

独立的纯前端 WebBluetooth 配网工作台，用于配套 `main` 当前 Rust server 的 JSON UART 协议。第一目标是让不方便安装 CLI / GUI 的用户也能通过浏览器完成设备 Wi-Fi 配网。

界面采用 Carbon Design System 风格：高密度顶部状态栏、左侧配网步骤、中间 Wi-Fi 表格、右侧确认面板和可展开 Debug trace。构建产物是纯静态 HTML/CSS/JS，不需要额外后端。

## 运行

```bash
pnpm install
pnpm dev
```

本地开发可以使用 `localhost`。生产部署必须使用 HTTPS，否则浏览器不会开放 Web Bluetooth。

## 静态发布

```bash
pnpm build
```

将生成的 `dist/` 目录放到任意 HTTPS 静态站点即可，例如 Nginx、Cloudflare Pages、Vercel 或 GitHub Pages。不要把 `dist/` 提交到仓库。

## 支持范围

- 支持桌面 Chrome/Edge 与 Android Chrome。
- iPhone/iPad、Safari、Firefox 等不支持 Web Bluetooth 的环境会显示不可用提示。
- 连接使用浏览器原生蓝牙设备选择器，不实现 CLI 式静默周边扫描列表。

## 配网流程

1. 打开 HTTPS 页面或本地 `localhost`。
2. 点击“连接设备”，在浏览器蓝牙选择器里选安装器提示过的 `yundrone-*` 设备，例如 `yundrone-12abcd`。
3. 点击“扫描 Wi-Fi”，等待扫描完成。
4. 选择扫描结果，或手动输入隐藏网络 SSID。
5. 输入密码并确认下发；开放网络可留空。
6. 等待 `wifi.provision` 返回成功或失败。

密码不会保存到浏览器存储。调试模式默认使用 Safe Verbose，会脱敏敏感字段。

## 验证

```bash
pnpm test
pnpm build
pnpm check:maxlines
```

`check:maxlines` 默认要求 `ts/tsx/scss/md` 文件不超过 250 行。

## 协议说明

本 worktree 从 `origin/main` 创建，因此 Web client 使用 main 当前 server 的协议：

- 请求是 JSON：`id/cmd/args/v`。
- 响应是 JSON：`id/cmd/phase/seq/final/ok/code/text/data/v`。
- 大响应使用 `data.chunk.mode=response_json` 分片。
- 客户端收到可靠 chunk 后发送 `link.ack` chunk ACK。
- 完整 response event 交付后发送 `link.ack` event ACK。
- 请求 accepted 阶段按 CLI 行为用同一 request id 重试，降低弱链路下的误失败率。

这里不使用 20B compact binary frame；Web 端与正式主线 server 保持 360B JSON chunking 协议一致。
