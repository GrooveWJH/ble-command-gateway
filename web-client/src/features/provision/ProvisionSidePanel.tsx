import {
  Button,
  InlineNotification,
  PasswordInput,
  TextInput,
  Tile,
} from "@carbon/react";

import { useI18n } from "../../i18n/useI18n";
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
  const { t } = useI18n();
  const busy = Boolean(props.busyCommand);
  const canProvision = props.connected && props.ssid.trim().length > 0 && !busy;
  return (
    <aside className="yd-side-panel">
      <h2>{t("provision.sideTitle")}</h2>
      <TextInput
        id="ssid"
        labelText="SSID"
        value={props.ssid}
        onChange={(event) => props.onSsidChange(event.target.value)}
        placeholder={t("provision.ssidPlaceholder")}
        disabled={!props.connected || busy}
      />
      <PasswordInput
        id="wifi-password"
        labelText={t("provision.passwordLabel")}
        value={props.password}
        onChange={(event) => props.onPasswordChange(event.target.value)}
        placeholder={t("provision.passwordPlaceholder")}
        disabled={!props.connected || busy}
        hidePasswordLabel={t("provision.hidePassword")}
        showPasswordLabel={t("provision.showPassword")}
      />
      <Tile className="yd-summary-tile">
        <strong>{t("provision.summary")}</strong>
        <span>{t("provision.summaryDevice", { device: props.deviceName ?? t("provision.notConnected") })}</span>
        <span>{t("provision.summarySsid", { ssid: props.ssid.trim() || t("provision.noSsid") })}</span>
        <span>{props.password ? t("provision.summaryPasswordSet") : t("provision.summaryPasswordEmpty")}</span>
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
          {t("provision.continue")}
        </Button>
      )}
      <Button onClick={props.onProvision} disabled={!canProvision}>
        {t("provision.submit")}
      </Button>
    </aside>
  );
}
