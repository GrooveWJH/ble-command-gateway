import { Search } from "@carbon/react";

import type { WifiNetwork } from "../../types";

export function WifiTable({
  connected,
  networks,
  filter,
  onFilterChange,
  onSelect,
}: {
  connected: boolean;
  networks: WifiNetwork[];
  filter: string;
  onFilterChange: (value: string) => void;
  onSelect: (ssid: string) => void;
}) {
  return (
    <div className="wifi-table">
      <div className="table-heading">
        <div>
          <h3>扫描结果</h3>
          <p>{emptyText(connected, networks.length)}</p>
        </div>
        <Search
          id="wifi-filter"
          labelText="搜索 SSID"
          value={filter}
          onChange={(event) => onFilterChange(event.target.value)}
          placeholder="搜索 SSID"
          disabled={!connected || networks.length === 0}
        />
      </div>
      <table aria-label="Wi-Fi 扫描结果">
        <thead>
          <tr><th>SSID</th><th>信号</th><th>信道</th></tr>
        </thead>
        <tbody>
          {networks.map((network, index) => (
            <tr key={`${network.ssid}-${index}`} onClick={() => onSelect(network.ssid)}>
              <td>{network.ssid}</td>
              <td>{network.signal}</td>
              <td>{network.channel}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function emptyText(connected: boolean, count: number): string {
  if (!connected) {
    return "先连接设备。尚未扫描 Wi-Fi。也可以直接在上方 SSID 输入框填写隐藏网络。";
  }
  return count === 0
    ? "尚未扫描 Wi-Fi。也可以直接在上方 SSID 输入框填写隐藏网络。"
    : `发现 ${count} 个候选网络，点击行即可选择。`;
}
