import {
  DataTable,
  Search,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableHeader,
  TableRow,
  TableToolbar,
  TableToolbarContent,
} from "@carbon/react";

import type { WifiNetwork } from "../../types";

const headers = [
  { key: "ssid", header: "SSID" },
  { key: "signal", header: "信号" },
  { key: "channel", header: "信道" },
  { key: "accessPoints", header: "接入点" },
];

interface MergedWifiNetwork {
  ssid: string;
  signal: number;
  channels: string[];
  accessPoints: number;
}

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
  const mergedNetworks = mergeWifiNetworks(networks);
  const rows = mergedNetworks.map((network) => ({
    id: network.ssid,
    ssid: network.ssid,
    signal: network.signal,
    channel: network.channels.join(" / "),
    accessPoints: `${network.accessPoints} 个接入点`,
  }));
  return (
    <DataTable rows={rows} headers={headers} size="sm" isSortable>
      {({ rows, headers, getHeaderProps, getRowProps, getTableProps }) => (
        <TableContainer title="扫描结果" description={emptyText(connected, mergedNetworks.length)}>
          <TableToolbar>
            <TableToolbarContent>
              <Search
                labelText="搜索 SSID"
                placeholder="搜索 SSID"
                value={filter}
                onChange={(event) => onFilterChange(event.target.value)}
                disabled={!connected || networks.length === 0}
              />
            </TableToolbarContent>
          </TableToolbar>
          <Table {...getTableProps()} aria-label="Wi-Fi 扫描结果">
            <TableHead>
              <TableRow>
                {headers.map((header) => (
                  <TableHeader {...getHeaderProps({ header })} key={header.key}>
                    {header.header}
                  </TableHeader>
                ))}
              </TableRow>
            </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow {...getRowProps({ row })} key={row.id} onClick={() => onSelect(row.cells[0].value as string)}>
                {row.cells.map((cell) => (
                    <TableCell key={cell.id}>
                      {cell.info.header === "signal"
                        ? <SignalQuality value={Number(cell.value)} />
                        : cell.value}
                    </TableCell>
                  ))}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      )}
    </DataTable>
  );
}

export function mergeWifiNetworks(networks: WifiNetwork[]): MergedWifiNetwork[] {
  const groups = new Map<string, MergedWifiNetwork>();
  for (const network of networks) {
    const current = groups.get(network.ssid);
    if (!current) {
      groups.set(network.ssid, {
        ssid: network.ssid,
        signal: network.signal,
        channels: [network.channel],
        accessPoints: 1,
      });
      continue;
    }
    current.signal = Math.max(current.signal, network.signal);
    current.accessPoints += 1;
    if (!current.channels.includes(network.channel)) {
      current.channels.push(network.channel);
    }
  }
  return [...groups.values()]
    .map((network) => ({ ...network, channels: sortChannels(network.channels) }))
    .sort((left, right) => right.signal - left.signal || left.ssid.localeCompare(right.ssid));
}

function sortChannels(channels: string[]): string[] {
  return [...channels].sort((left, right) => {
    const leftNumber = Number(left);
    const rightNumber = Number(right);
    if (Number.isFinite(leftNumber) && Number.isFinite(rightNumber)) {
      return leftNumber - rightNumber;
    }
    return left.localeCompare(right);
  });
}

function SignalQuality({ value }: { value: number }) {
  const quality = signalQuality(value);
  return (
    <span
      aria-label={`信号${quality.label}：${value}`}
      className={`yd-signal-quality yd-signal-quality--${quality.kind}`}
    >
      <span className="yd-signal-quality__dot" aria-hidden="true" />
      <span>{quality.label}</span>
      <span className="mono">{value}</span>
    </span>
  );
}

export function signalQuality(value: number): { kind: "strong" | "medium" | "weak"; label: "强" | "中" | "弱" } {
  if (value >= 0) {
    if (value >= 70) return { kind: "strong", label: "强" };
    if (value >= 40) return { kind: "medium", label: "中" };
    return { kind: "weak", label: "弱" };
  }
  if (value >= -60) return { kind: "strong", label: "强" };
  if (value >= -75) return { kind: "medium", label: "中" };
  return { kind: "weak", label: "弱" };
}

function emptyText(connected: boolean, count: number): string {
  if (!connected) {
    return "先连接设备。尚未扫描 Wi-Fi。也可以直接在上方 SSID 输入框填写隐藏网络。";
  }
  return count === 0
    ? "尚未扫描 Wi-Fi。也可以直接在上方 SSID 输入框填写隐藏网络。"
    : `发现 ${count} 个候选网络，点击行即可选择。`;
}
