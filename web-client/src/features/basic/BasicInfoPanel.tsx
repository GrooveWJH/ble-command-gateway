import { Button, CodeSnippet, Tag, Tile } from "@carbon/react";
import { useState } from "react";

import { capabilitiesView, heartbeatAlive, statusView } from "../../ui/diagnostics";
import type { CommandResponse, GatewayCommand } from "../../types";

export function BasicInfoPanel({
  busyCommand,
  statusResponse,
  capabilitiesResponse,
  heartbeatResponse,
  onStatus,
  onCapabilities,
  onHeartbeat,
}: {
  busyCommand?: GatewayCommand;
  statusResponse?: CommandResponse;
  capabilitiesResponse?: CommandResponse;
  heartbeatResponse?: CommandResponse;
  onStatus: () => void;
  onCapabilities: () => void;
  onHeartbeat: () => void;
}) {
  const [showRaw, setShowRaw] = useState(false);
  const [showProtocol, setShowProtocol] = useState(false);
  const status = statusView(statusResponse);
  const capabilities = capabilitiesView(capabilitiesResponse);
  const raw = statusResponse ?? capabilitiesResponse ?? heartbeatResponse;
  return (
    <section className="yd-utility-panel yd-basic-info-panel">
      <div className="yd-panel-heading">
        <div>
          <h2>基本信息</h2>
          <p>连接后自动读取被控端系统状态与协议能力；心跳保留为手动链路检测。</p>
        </div>
        <div className="yd-button-row">
          <Button size="sm" disabled={Boolean(busyCommand)} onClick={onStatus}>刷新系统状态</Button>
          <Button size="sm" kind="secondary" disabled={Boolean(busyCommand)} onClick={onCapabilities}>读取协议能力</Button>
          <Button size="sm" kind="ghost" disabled={Boolean(busyCommand)} onClick={onHeartbeat}>检测心跳</Button>
        </div>
      </div>
      <div className="yd-diagnostics-grid">
        <Tile className="yd-metric-card yd-metric-card--primary">
          <span>首选 IP</span>
          <strong>{status?.ip ?? "尚未刷新"}</strong>
          <p>{status?.network ?? "连接后会自动读取当前网络。"}</p>
        </Tile>
        <Tile className="yd-metric-card">
          <span>设备</span>
          <strong>{status?.deviceName ?? "未读取"}</strong>
          <p>{status ? `${status.hostname} · ${status.user}` : "系统状态会显示主机名与运行用户。"}</p>
        </Tile>
        <Tile className="yd-metric-card">
          <span>系统</span>
          <strong>{status?.system ?? "未读取"}</strong>
          <p>用于判断 NetworkManager、蓝牙服务与被控端运行环境。</p>
        </Tile>
        <Tile className="yd-metric-card">
          <span>协议版本</span>
          <strong>{capabilities?.protocolVersion ?? capabilitiesResponse?.v ?? "未读取"}</strong>
          <p>{capabilities ? `Payload limit：${capabilities.payloadLimit}` : "连接后会自动读取协议能力。"}</p>
        </Tile>
        <Tile className="yd-metric-card">
          <span>链路心跳</span>
          <strong>{heartbeatAlive(heartbeatResponse)}</strong>
          <p>{heartbeatResponse?.text || "手动检测 BLE UART request/response 是否正常。"}</p>
        </Tile>
      </div>
      {status && (
        <section className="yd-diagnostics-section">
          <h3>网络接口</h3>
          <div className="yd-interface-list">
            {status.interfaces.length === 0 ? (
              <Tile>未返回 IPv4 接口。</Tile>
            ) : status.interfaces.map((item) => (
              <Tile className="yd-interface-row" key={`${item.ifname}-${item.ipv4}`}>
                <strong>{item.ifname}</strong>
                <Tag type={item.kind === "wifi" ? "blue" : item.kind === "ethernet" ? "green" : "gray"}>
                  {item.kind}
                </Tag>
                <span className="mono">{item.ipv4}</span>
              </Tile>
            ))}
          </div>
        </section>
      )}
      {capabilities && (
        <details className="yd-advanced-protocol">
          <summary onClick={() => setShowProtocol((current) => !current)}>
            高级协议详情
          </summary>
          {showProtocol && (
            <section className="yd-diagnostics-section">
              <TagList title="命令" values={capabilities.commands} />
              <TagList title="特性" values={capabilities.features} />
            </section>
          )}
        </details>
      )}
      <details className="yd-raw-response" onToggle={(event) => setShowRaw(event.currentTarget.open)}>
        <summary>查看原始响应</summary>
        {showRaw && (
          <CodeSnippet type="multi" feedback="已复制">
            {JSON.stringify(raw ?? {}, null, 2)}
          </CodeSnippet>
        )}
      </details>
    </section>
  );
}

function TagList({ title, values }: { title: string; values: string[] }) {
  return (
    <div className="yd-tag-list">
      <span>{title}</span>
      <div>
        {values.length === 0 ? <Tag type="gray">未返回</Tag> : values.map((value) => (
          <Tag type="cool-gray" key={value}>{value}</Tag>
        ))}
      </div>
    </div>
  );
}
