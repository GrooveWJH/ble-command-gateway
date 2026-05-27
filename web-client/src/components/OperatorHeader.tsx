import {
  Header,
  HeaderGlobalBar,
  HeaderName,
  Tag,
  TextInput,
} from "@carbon/react";

import { PROTOCOL_VERSION } from "../protocol/commands";
import { busyLabel, connectionLabel } from "../ui/format";
import type { BrowserSupportState, GatewayState } from "../types";

export function OperatorHeader({
  support,
  state,
  targetPrefix,
  busy,
  onTargetPrefixChange,
}: {
  support: BrowserSupportState;
  state: GatewayState;
  targetPrefix: string;
  busy: boolean;
  onTargetPrefixChange: (value: string) => void;
}) {
  const connected = state.connection === "connected";
  const statusType = connected ? "green" : support.supported ? "gray" : "red";
  return (
    <Header aria-label="YunDrone WebBluetooth">
      <HeaderName href="#" prefix="YunDrone">
        配网工作台
      </HeaderName>
      <HeaderGlobalBar className="app-header__bar">
        <Tag type={statusType}>{connectionLabel(state.connection)}</Tag>
        <span className="mono status-text">{state.deviceName ?? "未选择设备"}</span>
        <Tag type={busy ? "blue" : "gray"}>{busyLabel(state.busyCommand)}</Tag>
        <span className="mono status-text">{PROTOCOL_VERSION}</span>
        <TextInput
          id="target-prefix"
          size="sm"
          labelText="目标前缀"
          hideLabel
          value={targetPrefix}
          onChange={(event) => onTargetPrefixChange(event.target.value)}
          disabled={connected || busy}
        />
      </HeaderGlobalBar>
    </Header>
  );
}
