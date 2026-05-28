import {
  Header,
  HeaderGlobalBar,
  HeaderName,
  Tag,
} from "@carbon/react";

import { PROTOCOL_VERSION } from "../protocol/commands";
import { busyLabel, connectionLabel } from "../ui/format";
import type { BrowserSupportState, GatewayState } from "../types";

export function OperatorHeader({
  support,
  state,
  targetPrefix,
  busy,
}: {
  support: BrowserSupportState;
  state: GatewayState;
  targetPrefix: string;
  busy: boolean;
}) {
  const connected = state.connection === "connected";
  const statusType = connected ? "green" : support.supported ? "gray" : "red";
  return (
    <Header aria-label="YunDrone WebBluetooth">
      <HeaderName href="#" prefix="YunDrone">
        配网工作台
      </HeaderName>
      <HeaderGlobalBar className="yd-header-status">
        <Tag type={statusType}>{connectionLabel(state.connection)}</Tag>
        <span className="mono yd-status-text">{state.deviceName ?? "未选择设备"}</span>
        <Tag type={busy ? "blue" : "gray"}>{busyLabel(state.busyCommand)}</Tag>
        <span className="mono yd-status-text">{PROTOCOL_VERSION}</span>
        <span className="mono yd-status-text">目标：{targetPrefix}</span>
      </HeaderGlobalBar>
    </Header>
  );
}
