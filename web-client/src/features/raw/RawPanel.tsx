import { Button, Select, SelectItem, TextArea } from "@carbon/react";

import { USER_COMMANDS, commandLabel } from "../../protocol/commands";
import type { GatewayCommand } from "../../types";

export function RawPanel({
  busyCommand,
  command,
  args,
  onCommandChange,
  onArgsChange,
  onSend,
}: {
  busyCommand?: GatewayCommand;
  command: GatewayCommand;
  args: string;
  onCommandChange: (command: GatewayCommand) => void;
  onArgsChange: (args: string) => void;
  onSend: () => void;
}) {
  const busy = Boolean(busyCommand);
  return (
    <section className="yd-utility-panel">
      <h2>高级命令</h2>
      <p>直接构造 JSON request，仍会自动处理 response_json chunk 和 link.ack。</p>
      <Select
        id="raw-command"
        labelText="命令"
        value={command}
        onChange={(event) => onCommandChange(event.target.value as GatewayCommand)}
      >
        {USER_COMMANDS.map((item) => (
          <SelectItem value={item} key={item} text={`${item} · ${commandLabel(item)}`} />
        ))}
      </Select>
      <TextArea
        id="raw-args"
        labelText="Args JSON"
        value={args}
        onChange={(event) => onArgsChange(event.target.value)}
        rows={8}
      />
      <Button disabled={busy} onClick={onSend}>发送高级命令</Button>
    </section>
  );
}
