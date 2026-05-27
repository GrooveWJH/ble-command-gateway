import { Button, Select, SelectItem, TextInput } from "@carbon/react";

import type { DebugMode, TraceEntry } from "../../types";

export function DebugConsole({
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
  const traceText = trace.map(formatTraceEntry).join("\n\n");
  return (
    <section className="debug-panel">
      <div className="debug-toolbar">
        <div>
          <h2>Debug trace</h2>
          <p>{trace.length} 条可见记录。默认 Safe Verbose 会脱敏密码字段。</p>
        </div>
        <Select
          id="debug-mode"
          labelText="调试模式"
          value={debugMode}
          onChange={(event) => onDebugModeChange(event.target.value as DebugMode)}
        >
          <SelectItem value="off" text="Off" />
          <SelectItem value="safe" text="Safe Verbose" />
          <SelectItem value="unsafe" text="Unsafe Raw" />
        </Select>
        <TextInput
          id="trace-filter"
          labelText="过滤 trace"
          placeholder="过滤 trace"
          value={traceFilter}
          onChange={(event) => onTraceFilterChange(event.target.value)}
        />
        <Button kind="secondary" onClick={() => onTracePausedChange(!tracePaused)}>
          {tracePaused ? "继续" : "暂停"}
        </Button>
        <Button kind="ghost" onClick={() => copyTrace(traceText)}>
          复制 trace
        </Button>
        <Button kind="ghost" onClick={() => exportTrace(traceText)}>
          导出 .trace
        </Button>
        <Button kind="ghost" onClick={onClear}>
          清空
        </Button>
        <Button
          kind="secondary"
          aria-expanded={expanded}
          onClick={() => onExpandedChange(!expanded)}
        >
          {expanded ? "收起" : "展开"}
        </Button>
      </div>
      {expanded && (
        <div className="trace-list" role="log">
          {trace.map((entry) => (
            <details className={`trace-entry trace-entry--${entry.level}`} key={entry.id} open={entry.level === "error"}>
              <summary>
                <span>{entry.at}</span>
                <strong>{entry.label}</strong>
                <span>{entry.summary}</span>
              </summary>
              {entry.detail && <pre>{entry.detail}</pre>}
            </details>
          ))}
        </div>
      )}
    </section>
  );
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
