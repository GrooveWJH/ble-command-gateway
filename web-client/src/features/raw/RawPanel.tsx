import { Button, Select, SelectItem, TextArea } from "@carbon/react";

import { commandLabel } from "../../protocol/commands";
import type { GatewayCommand } from "../../types";

const COMMANDS: GatewayCommand[] = [
  "link.heartbeat",
  "system.status",
  "system.capabilities",
  "wifi.scan",
  "wifi.provision",
  "wifi.profiles.list",
  "wifi.profiles.delete",
];

export function RawPanel({
  busy,
  command,
  args,
  onCommandChange,
  onArgsChange,
  onSend,
}: {
  busy: boolean;
  command: GatewayCommand;
  args: string;
  onCommandChange: (command: GatewayCommand) => void;
  onArgsChange: (args: string) => void;
  onSend: () => void;
}) {
  return (
    <section className="utility-panel">
      <h2>Raw 命令</h2>
      <p>直接构造 JSON request，仍会自动处理 response_json chunk 和 link.ack。</p>
      <Select
        id="raw-command"
        labelText="命令"
        value={command}
        onChange={(event) => onCommandChange(event.target.value as GatewayCommand)}
      >
        {COMMANDS.map((item) => (
          <SelectItem value={item} text={`${item} · ${commandLabel(item)}`} key={item} />
        ))}
      </Select>
      <TextArea
        id="raw-args"
        labelText="Args JSON"
        value={args}
        onChange={(event) => onArgsChange(event.target.value)}
        rows={8}
      />
      <Button disabled={busy} onClick={onSend}>发送 Raw 命令</Button>
    </section>
  );
}
