import { Button, InlineNotification, Tag, TextInput } from "@carbon/react";

import { useI18n } from "../i18n/useI18n";
import { browserSupportReason } from "../ui/browserSupport";
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
  const { t } = useI18n();
  const connected = state.connection === "connected";
  const supportReason = browserSupportReason(support, t);
  return (
    <section className="yd-command-center">
      <div className="yd-command-center__copy">
        <Tag type={support.supported ? "blue" : "red"}>
          {support.supported ? t("support.ready") : t("support.unavailable")}
        </Tag>
        <div>
          <h1>{t("support.title")}</h1>
          <p>{t("support.subtitle")}</p>
        </div>
      </div>
      <div className="yd-command-center__status" aria-label={t("support.statusAria")}>
        <span>{t("support.connectionStatus")}</span>
        <strong>{connectionLabel(state.connection, t)}</strong>
        <span className="mono">{state.deviceName ?? t("header.noDevice")}</span>
      </div>
      <div className="yd-command-center__actions">
        <TextInput
          id="target-prefix"
          labelText={t("support.targetPrefixLabel")}
          helperText={t("support.targetPrefixHelp")}
          value={targetPrefix}
          onChange={(event) => onTargetPrefixChange(event.target.value)}
          disabled={connected || busy}
        />
        <Button
          kind={connected ? "secondary" : "primary"}
          onClick={connected ? onDisconnect : onConnect}
          disabled={busy}
        >
          {connected ? t("support.disconnect") : t("support.connect")}
        </Button>
      </div>
      {!support.supported && (
        <InlineNotification
          kind="error"
          lowContrast
          title={t("support.unsupportedTitle")}
          subtitle={t("support.unsupportedSubtitle", { reason: supportReason })}
        />
      )}
    </section>
  );
}
