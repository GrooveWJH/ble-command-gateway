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
import type { TFunction } from "../../i18n/I18nProvider";

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
  t,
  onFilterChange,
  onSelect,
}: {
  connected: boolean;
  networks: WifiNetwork[];
  filter: string;
  t: TFunction;
  onFilterChange: (value: string) => void;
  onSelect: (ssid: string) => void;
}) {
  const headers = [
    { key: "ssid", header: "SSID" },
    { key: "signal", header: t("wifi.signal") },
    { key: "channel", header: t("wifi.channel") },
    { key: "accessPoints", header: t("wifi.accessPoints") },
  ];
  const mergedNetworks = mergeWifiNetworks(networks);
  const rows = mergedNetworks.map((network) => ({
    id: network.ssid,
    ssid: network.ssid,
    signal: network.signal,
    channel: network.channels.join(" / "),
    accessPoints: t("wifi.accessPointCount", { count: network.accessPoints }),
  }));
  return (
    <DataTable rows={rows} headers={headers} size="sm" isSortable>
      {({ rows, headers, getHeaderProps, getRowProps, getTableProps }) => (
        <TableContainer title={t("wifi.resultsTitle")} description={emptyText(connected, mergedNetworks.length, t)}>
          <TableToolbar>
            <TableToolbarContent>
              <Search
                labelText={t("wifi.search")}
                placeholder={t("wifi.search")}
                value={filter}
                onChange={(event) => onFilterChange(event.target.value)}
                disabled={!connected || networks.length === 0}
              />
            </TableToolbarContent>
          </TableToolbar>
          <Table {...getTableProps()} aria-label={t("wifi.aria")}>
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
                        ? <SignalQuality value={Number(cell.value)} t={t} />
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

function SignalQuality({ value, t }: { value: number; t: TFunction }) {
  const quality = signalQuality(value, t);
  return (
    <span
      aria-label={t("wifi.signalAria", { quality: quality.label, value })}
      className={`yd-signal-quality yd-signal-quality--${quality.kind}`}
    >
      <span className="yd-signal-quality__dot" aria-hidden="true" />
      <span>{quality.label}</span>
      <span className="mono">{value}</span>
    </span>
  );
}

export function signalQuality(value: number, t: TFunction): { kind: "strong" | "medium" | "weak"; label: string } {
  if (value >= 0) {
    if (value >= 70) return { kind: "strong", label: t("wifi.signalStrong") };
    if (value >= 40) return { kind: "medium", label: t("wifi.signalMedium") };
    return { kind: "weak", label: t("wifi.signalWeak") };
  }
  if (value >= -60) return { kind: "strong", label: t("wifi.signalStrong") };
  if (value >= -75) return { kind: "medium", label: t("wifi.signalMedium") };
  return { kind: "weak", label: t("wifi.signalWeak") };
}

function emptyText(connected: boolean, count: number, t: TFunction): string {
  if (!connected) {
    return t("wifi.emptyDisconnected");
  }
  return count === 0
    ? t("wifi.emptyConnected")
    : t("wifi.count", { count });
}
