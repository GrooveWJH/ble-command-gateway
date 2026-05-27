import {
  Button,
  StructuredListBody,
  StructuredListCell,
  StructuredListRow,
  StructuredListWrapper,
  Tag,
} from "@carbon/react";

import type { CapabilitiesResponseData, CommandResponse, HeartbeatResponseData, StatusResponseData } from "../../types";

export function DiagnosticsPanel(props: {
  busy: boolean;
  lastResponse?: CommandResponse;
  status?: StatusResponseData;
  capabilities?: CapabilitiesResponseData;
  heartbeat?: HeartbeatResponseData;
  onStatus: () => void;
  onCapabilities: () => void;
  onHeartbeat: () => void;
  onLinkCheck: () => void;
}) {
  return (
    <section className="utility-panel">
      <div className="panel-heading">
        <div><h2>诊断状态</h2><p>读取系统状态、协议能力与链路心跳。</p></div>
        <div className="button-row">
          <Button disabled={props.busy} onClick={props.onLinkCheck}>链路自检</Button>
          <Button disabled={props.busy} onClick={props.onStatus}>系统状态</Button>
          <Button kind="secondary" disabled={props.busy} onClick={props.onCapabilities}>协议能力</Button>
          <Button kind="ghost" disabled={props.busy} onClick={props.onHeartbeat}>心跳</Button>
        </div>
      </div>
      <div className="diagnostic-grid">
        <StatusSummary status={props.status} />
        <CapabilitiesSummary capabilities={props.capabilities} />
        <HeartbeatSummary heartbeat={props.heartbeat} />
      </div>
      <pre className="code-block">{props.lastResponse ? JSON.stringify(props.lastResponse, null, 2) : "暂无响应。"}</pre>
    </section>
  );
}

function StatusSummary({ status }: { status?: StatusResponseData }) {
  const interfaces = status?.interfaces ?? [];
  const rows = status ? [
    ["Device", status.device_name || "-"],
    ["Hostname", status.hostname || "-"],
    ["System", status.system || "-"],
    ["User", status.user || "-"],
    ["Network", status.network ?? "Not connected"],
    ["Preferred IP", status.ip ?? "Unavailable"],
    ...interfaces.map((item) => ["Interface", `${item.ifname} [${item.kind}] -> ${item.ipv4}`]),
  ] : [["Status", "尚未读取系统状态"]];
  return <SummaryList title="系统状态" rows={rows} />;
}

function CapabilitiesSummary({ capabilities }: { capabilities?: CapabilitiesResponseData }) {
  if (!capabilities) return <SummaryList title="协议能力" rows={[["Capabilities", "尚未读取协议能力"]]} />;
  const commands = capabilities.commands ?? [];
  const features = capabilities.features ?? [];
  return <SummaryList title="协议能力" rows={[
    ["Protocol", capabilities.protocol_version || "-"],
    ["Commands", commands.join(", ") || "-"],
    ["Features", features.join(", ") || "-"],
    ["JSON payload limit", `${capabilities.payload_limit} bytes`],
    ["Transport", "JSON UART + response_json chunks + link.ack"],
  ]} />;
}

function HeartbeatSummary({ heartbeat }: { heartbeat?: HeartbeatResponseData }) {
  return <div className="summary-tile"><strong>链路心跳</strong><Tag type={heartbeat?.alive ? "green" : "gray"}>{heartbeat?.alive ? "alive" : "尚未检测"}</Tag></div>;
}

function SummaryList({ title, rows }: { title: string; rows: string[][] }) {
  return (
    <div className="summary-tile">
      <strong>{title}</strong>
      <StructuredListWrapper>
        <StructuredListBody>
          {rows.map(([name, value], index) => (
            <StructuredListRow key={`${name}-${index}`}>
              <StructuredListCell>{name}</StructuredListCell>
              <StructuredListCell>{value}</StructuredListCell>
            </StructuredListRow>
          ))}
        </StructuredListBody>
      </StructuredListWrapper>
    </div>
  );
}
