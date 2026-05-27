import { Button, InlineNotification, Tag } from "@carbon/react";

import type { BrowserSupportState, GatewayState } from "../types";

export function SupportNotice({
  support,
  state,
  busy,
  onConnect,
  onDisconnect,
}: {
  support: BrowserSupportState;
  state: GatewayState;
  busy: boolean;
  onConnect: () => void;
  onDisconnect: () => void;
}) {
  const connected = state.connection === "connected";
  const needsLinuxChromeFlag =
    !support.hasBluetoothApi && /\bEdg\//.test(support.userAgent) && /Linux/.test(support.userAgent);
  const needsWindowsCheck =
    !support.hasBluetoothApi && /Windows NT/.test(support.userAgent);
  return (
    <section className="support-strip">
      <div>
        <Tag type={support.supported ? "blue" : "red"}>
          {support.supported ? "WebBluetooth ready" : "WebBluetooth unavailable"}
        </Tag>
        <h1>静态 HTTPS WebBluetooth 工具</h1>
        <p>无需安装 CLI。打开网页、连接面前已安装 server 的设备，然后完成 Wi-Fi 配网。</p>
      </div>
      <Button
        kind={connected ? "secondary" : "primary"}
        onClick={connected ? onDisconnect : onConnect}
        disabled={busy}
      >
        {connected ? "断开设备" : "连接设备"}
      </Button>
      {!support.supported && (
        <div className="support-diagnostics">
          <InlineNotification
            kind="error"
            lowContrast
            title="当前浏览器无法使用 Web Bluetooth"
            subtitle={support.reason}
          />
          {needsLinuxChromeFlag ? (
            <p>
              当前是 Linux Edge。请用 Google Chrome 打开{" "}
              <span className="mono">chrome://flags/#enable-experimental-web-platform-features</span>
              ，设为 Enabled 后重启 Chrome。
            </p>
          ) : needsWindowsCheck ? (
            <p>
              Windows 请使用最新版 Chrome/Edge，并确认系统蓝牙已开启。若仍为 false，
              打开 <span className="mono">chrome://flags/#enable-experimental-web-platform-features</span>
              或 <span className="mono">edge://flags/#enable-experimental-web-platform-features</span>
              ，设为 Enabled 后重启浏览器。
            </p>
          ) : null}
        </div>
      )}
    </section>
  );
}
