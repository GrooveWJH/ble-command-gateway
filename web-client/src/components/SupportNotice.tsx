import { Button, InlineNotification, Tag, TextInput } from "@carbon/react";

import { connectionLabel } from "../ui/format";
import type { BrowserSupportState, GatewayState } from "../types";

export function SupportNotice({
  support,
  state,
  busy,
  targetPrefix,
  onConnect,
  onDisconnect,
  onTargetPrefixChange,
}: {
  support: BrowserSupportState;
  state: GatewayState;
  busy: boolean;
  targetPrefix: string;
  onConnect: () => void;
  onDisconnect: () => void;
  onTargetPrefixChange: (value: string) => void;
}) {
  const connected = state.connection === "connected";
  return (
    <section className="yd-command-center">
      <div className="yd-command-center__copy">
        <Tag type={support.supported ? "blue" : "red"}>
          {support.supported ? "WebBluetooth ready" : "WebBluetooth unavailable"}
        </Tag>
        <div>
          <h1>BLE Wi-Fi 配网工作台</h1>
          <p>连接已部署被控端的 yundrone-* 设备，扫描 Wi-Fi 并下发配网；无需安装 CLI。</p>
        </div>
      </div>
      <div className="yd-command-center__status" aria-label="当前连接状态">
        <span>连接状态</span>
        <strong>{connectionLabel(state.connection)}</strong>
        <span className="mono">{state.deviceName ?? "尚未选择设备"}</span>
      </div>
      <div className="yd-command-center__actions">
        <TextInput
          id="target-prefix"
          labelText="目标设备前缀"
          helperText="默认扫描 yundrone-* 设备"
          value={targetPrefix}
          onChange={(event) => onTargetPrefixChange(event.target.value)}
          disabled={connected || busy}
        />
        <Button
          kind={connected ? "secondary" : "primary"}
          onClick={connected ? onDisconnect : onConnect}
          disabled={busy}
        >
          {connected ? "断开设备" : "连接设备"}
        </Button>
      </div>
      {!support.supported && (
        <InlineNotification
          kind="error"
          lowContrast
          title="当前浏览器无法使用 Web Bluetooth"
          subtitle={`${support.reason ?? "当前环境未开放 Web Bluetooth。"} 请使用 Google Chrome 或 Android Chrome。`}
        />
      )}
    </section>
  );
}
