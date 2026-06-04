import { Button, CodeSnippet, Tag, Tile } from "@carbon/react";
import { useState } from "react";

import { useI18n } from "../../i18n/useI18n";
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
  const { t } = useI18n();
  const [showRaw, setShowRaw] = useState(false);
  const [showProtocol, setShowProtocol] = useState(false);
  const status = statusView(statusResponse, t);
  const capabilities = capabilitiesView(capabilitiesResponse, t);
  const raw = statusResponse ?? capabilitiesResponse ?? heartbeatResponse;
  return (
    <section className="yd-utility-panel yd-basic-info-panel">
      <div className="yd-panel-heading">
        <div>
          <h2>{t("basic.title")}</h2>
          <p>{t("basic.subtitle")}</p>
        </div>
        <div className="yd-button-row">
          <Button size="sm" disabled={Boolean(busyCommand)} onClick={onStatus}>{t("basic.refreshStatus")}</Button>
          <Button size="sm" kind="secondary" disabled={Boolean(busyCommand)} onClick={onCapabilities}>{t("basic.readCapabilities")}</Button>
          <Button size="sm" kind="ghost" disabled={Boolean(busyCommand)} onClick={onHeartbeat}>{t("basic.checkHeartbeat")}</Button>
        </div>
      </div>
      <div className="yd-diagnostics-grid">
        <Tile className="yd-metric-card yd-metric-card--primary">
          <span>{t("basic.preferredIp")}</span>
          <strong>{status?.ip ?? t("basic.notRefreshed")}</strong>
          <p>{status?.network ?? t("basic.autoNetwork")}</p>
        </Tile>
        <Tile className="yd-metric-card">
          <span>{t("basic.device")}</span>
          <strong>{status?.deviceName ?? t("basic.notRead")}</strong>
          <p>{status ? `${status.hostname} · ${status.user}` : t("basic.deviceHelp")}</p>
        </Tile>
        <Tile className="yd-metric-card">
          <span>{t("basic.system")}</span>
          <strong>{status?.system ?? t("basic.notRead")}</strong>
          <p>{t("basic.systemHelp")}</p>
        </Tile>
        <Tile className="yd-metric-card">
          <span>{t("basic.protocolVersion")}</span>
          <strong>{capabilities?.protocolVersion ?? capabilitiesResponse?.v ?? t("basic.notRead")}</strong>
          <p>{capabilities ? t("basic.payloadLimit", { limit: capabilities.payloadLimit }) : t("basic.protocolHelp")}</p>
        </Tile>
        <Tile className="yd-metric-card">
          <span>{t("basic.heartbeat")}</span>
          <strong>{heartbeatAlive(heartbeatResponse, t)}</strong>
          <p>{heartbeatResponse?.text || t("basic.heartbeatHelp")}</p>
        </Tile>
      </div>
      {status && (
        <section className="yd-diagnostics-section">
          <h3>{t("basic.interfaces")}</h3>
          <div className="yd-interface-list">
            {status.interfaces.length === 0 ? (
              <Tile>{t("basic.noIpv4")}</Tile>
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
            {t("basic.advancedProtocol")}
          </summary>
          {showProtocol && (
            <section className="yd-diagnostics-section">
              <TagList title={t("basic.commands")} values={capabilities.commands} />
              <TagList title={t("basic.features")} values={capabilities.features} />
            </section>
          )}
        </details>
      )}
      <details className="yd-raw-response" onToggle={(event) => setShowRaw(event.currentTarget.open)}>
        <summary>{t("basic.viewRaw")}</summary>
        {showRaw && (
          <CodeSnippet type="multi" feedback={t("install.copied")}>
            {JSON.stringify(raw ?? {}, null, 2)}
          </CodeSnippet>
        )}
      </details>
    </section>
  );
}

function TagList({ title, values }: { title: string; values: string[] }) {
  const { t } = useI18n();
  return (
    <div className="yd-tag-list">
      <span>{title}</span>
      <div>
        {values.length === 0 ? <Tag type="gray">{t("basic.noneReturned")}</Tag> : values.map((value) => (
          <Tag type="cool-gray" key={value}>{value}</Tag>
        ))}
      </div>
    </div>
  );
}
