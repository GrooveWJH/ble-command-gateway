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
];

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
  const rows = networks.map((network, index) => ({
    id: `${network.ssid}-${index}`,
    ssid: network.ssid,
    signal: network.signal,
    channel: network.channel,
  }));
  return (
    <DataTable rows={rows} headers={headers} size="sm" isSortable>
      {({ rows, headers, getHeaderProps, getRowProps, getTableProps }) => (
        <TableContainer title="扫描结果" description={emptyText(connected, networks.length)}>
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
                    <TableCell key={cell.id}>{cell.value}</TableCell>
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

function emptyText(connected: boolean, count: number): string {
  if (!connected) {
    return "先连接设备。尚未扫描 Wi-Fi。也可以直接在上方 SSID 输入框填写隐藏网络。";
  }
  return count === 0
    ? "尚未扫描 Wi-Fi。也可以直接在上方 SSID 输入框填写隐藏网络。"
    : `发现 ${count} 个候选网络，点击行即可选择。`;
}
