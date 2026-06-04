import {
  Header,
  HeaderGlobalAction,
  HeaderGlobalBar,
  HeaderName,
  Tag,
} from "@carbon/react";

import { InstallScriptCallout } from "./InstallScriptCallout";
import { APP_VERSION } from "../appVersion";
import { useI18n } from "../i18n/useI18n";
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
  const { t, toggleLanguage } = useI18n();
  const connected = state.connection === "connected";
  const statusType = connected ? "green" : support.supported ? "gray" : "red";
  return (
    <Header aria-label={t("header.ariaLabel")}>
      <HeaderName href="#" prefix={t("header.brandPrefix")}>
        {t("header.brandName")}
      </HeaderName>
      <InstallScriptCallout />
      <HeaderGlobalBar className="yd-header-actions">
        <div className="yd-header-status">
          <Tag type={statusType}>{connectionLabel(state.connection, t)}</Tag>
          <span className="mono yd-status-text">{state.deviceName ?? t("header.noDevice")}</span>
          <Tag type={busy ? "blue" : "gray"}>{busyLabel(state.busyCommand, t)}</Tag>
          <span className="mono yd-status-text">{t("header.releaseVersion", { version: APP_VERSION })}</span>
          <span className="mono yd-status-text">{t("header.targetPrefix", { prefix: targetPrefix })}</span>
        </div>
        <HeaderGlobalAction
          aria-label={t("header.languageSwitchLabel")}
          className="yd-language-action"
          onClick={toggleLanguage}
          tooltipAlignment="end"
        >
          <span aria-hidden="true" className="yd-language-action__badge">
            {t("header.languageBadge")}
          </span>
        </HeaderGlobalAction>
      </HeaderGlobalBar>
    </Header>
  );
}
