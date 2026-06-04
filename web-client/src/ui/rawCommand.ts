import { traceEntry } from "../services/trace";
import type { TFunction } from "../i18n/I18nProvider";
import type { GatewayCommand, JsonObject, TraceEntry } from "../types";

export function sendRawCommand(
  command: GatewayCommand,
  args: string,
  run: (cmd: GatewayCommand, args?: JsonObject) => void,
  trace: (entry: TraceEntry) => void,
  t: TFunction,
) {
  try {
    run(command, JSON.parse(args) as JsonObject);
  } catch {
    trace(traceEntry("ERR", t("raw.invalidArgs"), undefined, "error"));
  }
}
