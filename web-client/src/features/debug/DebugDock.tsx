import { Button, Select, SelectItem, TextInput } from "@carbon/react";

import { useI18n } from "../../i18n/useI18n";
import type { DebugMode, TraceEntry } from "../../types";

export function DebugDock({
  debugMode,
  traceFilter,
  tracePaused,
  expanded,
  trace,
  onDebugModeChange,
  onTraceFilterChange,
  onTracePausedChange,
  onExpandedChange,
  onClear,
}: {
  debugMode: DebugMode;
  traceFilter: string;
  tracePaused: boolean;
  expanded: boolean;
  trace: TraceEntry[];
  onDebugModeChange: (mode: DebugMode) => void;
  onTraceFilterChange: (value: string) => void;
  onTracePausedChange: (paused: boolean) => void;
  onExpandedChange: (expanded: boolean) => void;
  onClear: () => void;
}) {
  const { t } = useI18n();
  const traceText = trace.map(formatTraceEntry).join("\n\n");
  return (
    <section className="yd-debug-dock" aria-label={t("debug.aria")}>
      {expanded && (
        <div className="yd-debug-drawer">
          <div className="yd-debug-toolbar">
            <div className="yd-debug-toolbar__summary">
              <h2>{t("debug.title")}</h2>
              <p>{t("debug.summary", { count: trace.length })}</p>
            </div>
            <div className="yd-debug-toolbar__filters">
              <Select
                id="debug-mode"
                labelText={t("debug.mode")}
                value={debugMode}
                onChange={(event) => onDebugModeChange(event.target.value as DebugMode)}
              >
                <SelectItem value="off" text={debugModeLabel("off", t)} />
                <SelectItem value="safe" text={debugModeLabel("safe", t)} />
                <SelectItem value="unsafe" text={debugModeLabel("unsafe", t)} />
              </Select>
              <TextInput
                id="trace-filter"
                labelText={t("debug.filter")}
                placeholder={t("debug.filter")}
                value={traceFilter}
                onChange={(event) => onTraceFilterChange(event.target.value)}
              />
            </div>
            <div className="yd-debug-toolbar__actions">
              <Button kind="secondary" onClick={() => onTracePausedChange(!tracePaused)}>
                {tracePaused ? t("debug.resume") : t("debug.pause")}
              </Button>
              <Button kind="ghost" onClick={() => copyTrace(traceText)}>
                {t("debug.copy")}
              </Button>
              <Button kind="ghost" onClick={() => exportTrace(traceText)}>
                {t("debug.export")}
              </Button>
              <Button kind="ghost" onClick={onClear}>
                {t("debug.clear")}
              </Button>
            </div>
          </div>
          <div className="yd-trace-list" role="log">
            {trace.length === 0 ? (
              <div className="yd-trace-empty">{t("debug.empty")}</div>
            ) : trace.map((entry) => (
              <details className={`yd-trace-entry yd-trace-entry--${entry.level}`} key={entry.id} open={entry.level === "error"}>
                <summary>
                  <span>{entry.at}</span>
                  <strong>{entry.label}</strong>
                  <span>{entry.summary}</span>
                </summary>
                {entry.detail && <pre>{entry.detail}</pre>}
              </details>
            ))}
          </div>
        </div>
      )}
      <div className="yd-debug-bar">
        <div>
          <strong>{t("debug.records", { count: trace.length })}</strong>
          <span>{t("debug.modeLabel", { mode: debugModeLabel(debugMode, t) })}</span>
          <span>{tracePaused ? t("debug.paused") : t("debug.live")}</span>
        </div>
        <Button
          size="sm"
          kind="ghost"
          aria-expanded={expanded}
          onClick={() => onExpandedChange(!expanded)}
        >
          {expanded ? t("debug.collapse") : t("debug.expand")}
        </Button>
      </div>
    </section>
  );
}

function debugModeLabel(mode: DebugMode, t: ReturnType<typeof useI18n>["t"]): string {
  switch (mode) {
    case "off":
      return t("debug.modeOff");
    case "safe":
      return t("debug.modeSafe");
    case "unsafe":
      return t("debug.modeUnsafe");
  }
}

function formatTraceEntry(entry: TraceEntry): string {
  return `[${entry.at}] ${entry.level.toUpperCase()} ${entry.label} ${entry.summary}${entry.detail ? `\n${entry.detail}` : ""}`;
}

function copyTrace(text: string): void {
  void globalThis.navigator?.clipboard?.writeText(text);
}

function exportTrace(text: string): void {
  const blob = new Blob([text], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `yundrone-${new Date().toISOString().replace(/[:.]/g, "-")}.trace`;
  link.click();
  URL.revokeObjectURL(url);
}
