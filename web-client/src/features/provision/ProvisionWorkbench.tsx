import { Button, InlineNotification } from "@carbon/react";

import { WifiTable } from "./WifiTable";
import { ProvisionSidePanel } from "./ProvisionSidePanel";
import type {
  ProvisionResultView,
  UserFacingError,
  WifiNetwork,
} from "../../types";

export function ProvisionWorkbench(props: {
  connected: boolean;
  busy: boolean;
  deviceName?: string;
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
  const filtered = props.networks.filter((network) =>
    network.ssid.toLowerCase().includes(props.networkFilter.trim().toLowerCase()),
  );
  return (
    <div className="workbench">
      <section className="main-panel">
        <div className="panel-heading">
          <div>
            <h2>配网工作台</h2>
            <p>先连接设备，再扫描 Wi-Fi。无法扫描到隐藏网络时，可直接手动输入 SSID。</p>
          </div>
          <Button onClick={props.onScan} disabled={props.busy || !props.connected}>
            {props.busy ? "正在执行..." : "扫描 Wi-Fi"}
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
  );
}
