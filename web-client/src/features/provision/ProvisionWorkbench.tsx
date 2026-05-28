import { Button, InlineNotification, Layer } from "@carbon/react";

import { WifiTable } from "./WifiTable";
import { ProvisionSidePanel } from "./ProvisionSidePanel";
import { ProvisionSteps } from "../../components/ProvisionSteps";
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
              <h2>Wi-Fi 配网</h2>
              <p>先连接设备，再扫描 Wi-Fi。无法扫描到隐藏网络时，可直接手动输入 SSID。</p>
            </div>
            <Button onClick={props.onScan} disabled={busy || !props.connected}>
              扫描 Wi-Fi
            </Button>
          </div>
          {!props.connected && (
            <InlineNotification
              kind="warning"
              lowContrast
              title="先连接设备"
              subtitle="点击右上角连接设备，在浏览器蓝牙选择器中选择名称以 yundrone- 开头的设备。设备保持上电并靠近电脑或手机。"
            />
          )}
          <WifiTable
            connected={props.connected}
            networks={filtered}
            filter={props.networkFilter}
            onFilterChange={props.onNetworkFilterChange}
            onSelect={props.onSsidChange}
          />
        </section>
        <ProvisionSidePanel {...props} />
      </div>
    </Layer>
  );
}
