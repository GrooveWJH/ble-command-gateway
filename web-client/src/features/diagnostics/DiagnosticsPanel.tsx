import { Button, CodeSnippet } from "@carbon/react";

import type { CommandResponse } from "../../types";

export function DiagnosticsPanel({
  busy,
  lastResponse,
  onStatus,
  onCapabilities,
  onHeartbeat,
}: {
  busy: boolean;
  lastResponse?: CommandResponse;
  onStatus: () => void;
  onCapabilities: () => void;
  onHeartbeat: () => void;
}) {
  return (
    <section className="utility-panel">
      <div className="panel-heading">
        <div>
          <h2>诊断状态</h2>
          <p>读取系统状态、协议能力与链路心跳。</p>
        </div>
        <div className="button-row">
          <Button size="sm" disabled={busy} onClick={onStatus}>系统状态</Button>
          <Button size="sm" kind="secondary" disabled={busy} onClick={onCapabilities}>协议能力</Button>
          <Button size="sm" kind="ghost" disabled={busy} onClick={onHeartbeat}>心跳</Button>
        </div>
      </div>
      <CodeSnippet type="multi" feedback="已复制">
        {lastResponse ? JSON.stringify(lastResponse, null, 2) : "暂无响应。"}
      </CodeSnippet>
    </section>
  );
}
