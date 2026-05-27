import {
  Button,
  InlineNotification,
  PasswordInput,
  TextInput,
  Tile,
} from "@carbon/react";

import type { ProvisionResultView, UserFacingError } from "../../types";

export function ProvisionSidePanel(props: {
  connected: boolean;
  busy: boolean;
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
  const canProvision = props.connected && props.ssid.trim().length > 0 && !props.busy;
  return (
    <aside className="side-panel">
      <h2>下发配网</h2>
      <TextInput
        id="ssid"
        labelText="SSID"
        value={props.ssid}
        onChange={(event) => props.onSsidChange(event.target.value)}
        placeholder="选择扫描结果，或手动输入隐藏网络 SSID"
        disabled={!props.connected || props.busy}
      />
      <PasswordInput
        id="wifi-password"
        labelText="Wi-Fi 密码"
        value={props.password}
        onChange={(event) => props.onPasswordChange(event.target.value)}
        placeholder="开放网络可留空"
        disabled={!props.connected || props.busy}
        hidePasswordLabel="隐藏密码"
        showPasswordLabel="显示密码"
      />
      <Tile className="summary-tile">
        <strong>确认摘要</strong>
        <span>设备：{props.deviceName ?? "尚未连接"}</span>
        <span>目标 SSID：{props.ssid.trim() || "尚未选择"}</span>
        <span>密码：{props.password ? "已填写，不会保存到浏览器" : "未填写，适用于开放网络"}</span>
      </Tile>
      {props.busy && (
        <InlineNotification
          kind="info"
          lowContrast
          title="命令执行中"
          subtitle="请保持页面打开、设备上电并靠近当前浏览器。Wi-Fi 扫描和配网通常需要 10 到 30 秒。"
        />
      )}
      {props.error && (
        <InlineNotification
          kind="error"
          lowContrast
          title={props.error.title}
          subtitle={`${props.error.subtitle} ${props.error.nextStep}`}
        />
      )}
      {props.result && (
        <ResultNotice result={props.result} onReset={props.onResetResult} />
      )}
      <Button onClick={props.onProvision} disabled={!canProvision}>
        确认并下发配网
      </Button>
    </aside>
  );
}

function ResultNotice({
  result,
  onReset,
}: {
  result: ProvisionResultView;
  onReset: () => void;
}) {
  return (
    <div className="result-stack">
      <InlineNotification
        kind={result.ok ? "success" : "error"}
        lowContrast
        title={result.ok ? "配网成功" : `配网失败：${result.code}`}
        subtitle={result.ip ? `${result.text} IP: ${result.ip}` : result.text}
      />
      <Button kind="tertiary" size="sm" onClick={onReset}>
        继续配置其他网络
      </Button>
    </div>
  );
}
