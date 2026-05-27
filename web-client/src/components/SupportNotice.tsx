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
        disabled={!support.supported || busy}
      >
        {connected ? "断开设备" : "连接设备"}
      </Button>
      {!support.supported && (
        <InlineNotification
          kind="error"
          lowContrast
          title="当前浏览器无法使用 Web Bluetooth"
          subtitle={support.reason}
        />
      )}
    </section>
  );
}
