import {
  Button,
  InlineNotification,
  PasswordInput,
  TextInput,
  Tile,
} from "@carbon/react";

import type { GatewayCommand, ProvisionResultView, UserFacingError } from "../../types";

export function ProvisionSidePanel(props: {
  connected: boolean;
  busyCommand?: GatewayCommand;
  deviceName?: string;
  ssid: string;
  password: string;
  result?: ProvisionResultView;
  error?: UserFacingError;
  onSsidChange: (value: string) => void;
  onPasswordChange: (value: string) => void;
  onProvision: () => void;
  onResetResult: () => void;
}) {
  const busy = Boolean(props.busyCommand);
  const canProvision = props.connected && props.ssid.trim().length > 0 && !busy;
  return (
    <aside className="yd-side-panel">
      <h2>下发配网</h2>
      <TextInput
        id="ssid"
        labelText="SSID"
        value={props.ssid}
        onChange={(event) => props.onSsidChange(event.target.value)}
        placeholder="选择扫描结果，或手动输入隐藏网络 SSID"
        disabled={!props.connected || busy}
      />
      <PasswordInput
        id="wifi-password"
        labelText="Wi-Fi 密码"
        value={props.password}
        onChange={(event) => props.onPasswordChange(event.target.value)}
        placeholder="开放网络可留空"
        disabled={!props.connected || busy}
        hidePasswordLabel="隐藏密码"
        showPasswordLabel="显示密码"
      />
      <Tile className="yd-summary-tile">
        <strong>确认摘要</strong>
        <span>设备：{props.deviceName ?? "尚未连接"}</span>
        <span>目标 SSID：{props.ssid.trim() || "尚未选择"}</span>
        <span>密码：{props.password ? "已填写，不会保存到浏览器" : "未填写，适用于开放网络"}</span>
      </Tile>
      {props.error && (
        <InlineNotification
          kind="error"
          lowContrast
          title={props.error.title}
          subtitle={`${props.error.subtitle} ${props.error.nextStep}`}
        />
      )}
      {props.result && (
        <Button kind="tertiary" size="sm" onClick={props.onResetResult}>
          继续配置其他网络
        </Button>
      )}
      <Button onClick={props.onProvision} disabled={!canProvision}>
        确认并下发配网
      </Button>
    </aside>
  );
}
