import { Button, InlineNotification, Layer } from "@carbon/react";

import { WifiTable } from "./WifiTable";
import { ProvisionSidePanel } from "./ProvisionSidePanel";
import { ProvisionSteps } from "../../components/ProvisionSteps";
import { useI18n } from "../../i18n/useI18n";
import type {
  GatewayCommand,
  ProvisionJourneyStep,
  ProvisionResultView,
  UserFacingError,
  WifiNetwork,
} from "../../types";

export function ProvisionWorkbench(props: {
  connected: boolean;
  busyCommand?: GatewayCommand;
  deviceName?: string;
  journeyStep: ProvisionJourneyStep;
  networks: WifiNetwork[];
  ssid: string;
  password: string;
  networkFilter: string;
  result?: ProvisionResultView;
  error?: UserFacingError;
  onSsidChange: (value: string) => void;
  onPasswordChange: (value: string) => void;
  onNetworkFilterChange: (value: string) => void;
  onScan: () => void;
  onProvision: () => void;
  onResetResult: () => void;
}) {
  const { t } = useI18n();
  const busy = Boolean(props.busyCommand);
  const filtered = props.networks.filter((network) =>
    network.ssid.toLowerCase().includes(props.networkFilter.trim().toLowerCase()),
  );
  return (
    <Layer className="yd-workbench">
      <ProvisionSteps current={props.journeyStep} />
      <div className="yd-workbench-grid">
        <section className="yd-main-panel">
          <div className="yd-panel-heading">
            <div>
              <h2>{t("provision.title")}</h2>
              <p>{t("provision.subtitle")}</p>
            </div>
            <Button onClick={props.onScan} disabled={busy || !props.connected}>
              {t("provision.scan")}
            </Button>
          </div>
          {!props.connected && (
            <InlineNotification
              kind="warning"
              lowContrast
              title={t("provision.connectFirstTitle")}
              subtitle={t("provision.connectFirstSubtitle")}
            />
          )}
          <WifiTable
            connected={props.connected}
            networks={filtered}
            filter={props.networkFilter}
            t={t}
            onFilterChange={props.onNetworkFilterChange}
            onSelect={props.onSsidChange}
          />
        </section>
        <ProvisionSidePanel {...props} />
      </div>
    </Layer>
  );
}
