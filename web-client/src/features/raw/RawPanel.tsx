import { Button, Select, SelectItem, TextArea } from "@carbon/react";

import { useI18n } from "../../i18n/useI18n";
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
  const { t } = useI18n();
  const busy = Boolean(busyCommand);
  return (
    <section className="yd-utility-panel">
      <h2>{t("raw.title")}</h2>
      <p>{t("raw.subtitle")}</p>
      <Select
        id="raw-command"
        labelText={t("raw.command")}
        value={command}
        onChange={(event) => onCommandChange(event.target.value as GatewayCommand)}
      >
        {USER_COMMANDS.map((item) => (
          <SelectItem value={item} key={item} text={`${item} · ${commandLabel(item, t)}`} />
        ))}
      </Select>
      <TextArea
        id="raw-args"
        labelText={t("raw.args")}
        value={args}
        onChange={(event) => onArgsChange(event.target.value)}
        rows={8}
      />
      <Button disabled={busy} onClick={onSend}>{t("raw.send")}</Button>
    </section>
  );
}
