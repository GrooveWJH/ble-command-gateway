import { Content, Theme } from "@carbon/react";
import { useEffect, useMemo, useRef, useState } from "react";

import { getBrowserSupport, requestYundroneDevice } from "./ble/webBluetooth";
import { JourneyRail } from "./components/JourneyRail";
import { OperatorHeader } from "./components/OperatorHeader";
import { SupportNotice } from "./components/SupportNotice";
import { DebugConsole } from "./features/debug/DebugConsole";
import { DiagnosticsPanel } from "./features/diagnostics/DiagnosticsPanel";
import { ProfilesPanel } from "./features/profiles/ProfilesPanel";
import { ProvisionWorkbench } from "./features/provision/ProvisionWorkbench";
import { RawPanel } from "./features/raw/RawPanel";
import { GatewayClient } from "./services/GatewayClient";
import { traceEntry } from "./services/trace";
import { explainError } from "./ui/errors";
import { currentJourneyStep } from "./ui/journey";
import { safeStorageGet, safeStorageSet } from "./ui/storage";
import type {
  CommandResponse,
  CapabilitiesResponseData,
  DebugMode,
  GatewayCommand,
  GatewayState,
  HeartbeatResponseData,
  JsonObject,
  StatusResponseData,
  TraceEntry,
  UserFacingError,
  WifiNetwork,
  WifiProfile,
  WifiProfilesDeleteResponseData,
} from "./types";

interface AppProps {
  initialTrace?: TraceEntry[];
}

type PanelId = "provision" | "diagnostics" | "profiles" | "raw";

const DEFAULT_TRACE = [
  traceEntry("SYS", "Web console ready. 使用 Google Chrome 或 Android Chrome 连接设备。"),
];

export default function App({ initialTrace }: AppProps) {
  const support = useMemo(() => getBrowserSupport(), []);
  const clientRef = useRef<GatewayClient | null>(null);
  const [debugMode, setDebugMode] = useState<DebugMode>((safeStorageGet("debugMode") as DebugMode | null) ?? "safe");
  const [targetPrefix, setTargetPrefix] = useState(safeStorageGet("targetPrefix") ?? "yundrone");
  const [state, setState] = useState<GatewayState>({
    connection: support.supported ? "idle" : "unsupported",
    trace: initialTrace ?? DEFAULT_TRACE,
  });
  const [networks, setNetworks] = useState<WifiNetwork[]>([]);
  const [profiles, setProfiles] = useState<WifiProfile[]>([]);
  const [statusData, setStatusData] = useState<StatusResponseData | undefined>();
  const [capabilitiesData, setCapabilitiesData] = useState<CapabilitiesResponseData | undefined>();
  const [heartbeatData, setHeartbeatData] = useState<HeartbeatResponseData | undefined>();
  const [profileDeleteResult, setProfileDeleteResult] = useState<WifiProfilesDeleteResponseData | undefined>();
  const [ssid, setSsid] = useState("");
  const [password, setPassword] = useState("");
  const [networkFilter, setNetworkFilter] = useState("");
  const [result, setResult] = useState<CommandResponse | undefined>();
  const [error, setError] = useState<UserFacingError | undefined>();
  const [selectedProfiles, setSelectedProfiles] = useState<string[]>([]);
  const [rawCommand, setRawCommand] = useState<GatewayCommand>("system.status");
  const [rawArgs, setRawArgs] = useState("{}");
  const [activePanel, setActivePanel] = useState<PanelId>("provision");
  const [traceFilter, setTraceFilter] = useState("");
  const [tracePaused, setTracePaused] = useState(false);
  const [traceExpanded, setTraceExpanded] = useState(safeStorageGet("traceExpanded") === "true");

  useEffect(() => {
    safeStorageSet("debugMode", debugMode);
    clientRef.current?.setDebugMode(debugMode);
  }, [debugMode]);

  useEffect(() => safeStorageSet("targetPrefix", targetPrefix), [targetPrefix]);
  useEffect(() => safeStorageSet("traceExpanded", String(traceExpanded)), [traceExpanded]);

  const addTrace = (entry: TraceEntry) => {
    if (!tracePaused) {
      setState((current) => ({ ...current, trace: [...current.trace.slice(-499), entry] }));
    }
  };

  const connect = async () => {
    if (!support.supported) {
      const message = support.reason ?? "当前浏览器无法使用 Web Bluetooth。";
      setError({
        title: "当前浏览器不可用",
        subtitle: message,
        nextStep: "请使用支持 Web Bluetooth 的浏览器后再试。",
      });
      addTrace(traceEntry("ERR", message, undefined, "error"));
      return;
    }
    setState((current) => ({ ...current, connection: "connecting" }));
    try {
      const connection = await requestYundroneDevice(targetPrefix);
      clientRef.current = new GatewayClient(connection, {
        debugMode,
        onTrace: addTrace,
        onEvent: (response) => setState((current) => ({ ...current, lastResponse: response })),
        onDisconnect: () => handleDisconnected(),
      });
      setError(undefined);
      setState((current) => ({ ...current, connection: "connected", deviceName: connection.device.name ?? "YunDrone BLE" }));
      addTrace(traceEntry("SYS", `Connected to ${connection.device.name ?? "YunDrone BLE"}`));
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setError(explainError(message));
      setState((current) => ({ ...current, connection: "error" }));
      addTrace(traceEntry("ERR", message, undefined, "error"));
    }
  };

  const handleDisconnected = () => {
    clientRef.current = null;
    setError(explainError("Device disconnected"));
    setState((current) => ({ ...current, connection: support.supported ? "idle" : "unsupported", deviceName: undefined, busyCommand: undefined }));
  };

  const disconnect = () => {
    clientRef.current?.disconnect();
    handleDisconnected();
    addTrace(traceEntry("SYS", "Disconnected"));
  };

  const runLinkCheck = async () => {
    for (const cmd of ["link.heartbeat", "system.capabilities", "system.status"] as const) {
      const response = await runCommand(cmd);
      if (!response?.ok) break;
    }
  };

  const runCommand = async (cmd: GatewayCommand, args: JsonObject = {}) => {
    if (!clientRef.current) {
      const message = "请先点击连接设备，在浏览器蓝牙选择器里选择 yundrone-* 设备。";
      setError(explainError(message));
      addTrace(traceEntry("ERR", message, undefined, "error"));
      return undefined;
    }
    setState((current) => ({ ...current, busyCommand: cmd }));
    const started = performance.now();
    try {
      const response = await clientRef.current.runCommand(cmd, args);
      addTrace(traceEntry("QOS:elapsed", `cmd=${cmd} id=${response.id} elapsed=${Math.round(performance.now() - started)}ms`));
      applyResponse(cmd, response);
      setError(undefined);
      return response;
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setError(explainError(message));
      addTrace(traceEntry("ERR", message, undefined, "error"));
      return undefined;
    } finally {
      setState((current) => ({ ...current, busyCommand: undefined }));
    }
  };

  const applyResponse = (cmd: GatewayCommand, response: CommandResponse) => {
    setResult(response);
    if (cmd === "wifi.scan") {
      const loaded = Array.isArray(response.data?.networks) ? response.data.networks as unknown as WifiNetwork[] : [];
      setNetworks([...loaded].sort((left, right) => right.signal - left.signal));
      setNetworkFilter("");
    }
    if (cmd === "system.status" && response.ok) {
      setStatusData(response.data as unknown as StatusResponseData);
    }
    if (cmd === "system.capabilities" && response.ok) {
      setCapabilitiesData(response.data as unknown as CapabilitiesResponseData);
    }
    if (cmd === "link.heartbeat" && response.ok) {
      setHeartbeatData(response.data as unknown as HeartbeatResponseData);
    }
    if (cmd === "wifi.provision" && !response.ok) setError(explainError(response.text));
    if (cmd === "wifi.profiles.list") {
      const loaded = Array.isArray(response.data?.profiles) ? response.data.profiles as unknown as WifiProfile[] : [];
      setProfiles(loaded);
      setProfileDeleteResult(undefined);
    }
    if (cmd === "wifi.profiles.delete") {
      setProfileDeleteResult(response.data as unknown as WifiProfilesDeleteResponseData);
      setSelectedProfiles([]);
      void runCommand("wifi.profiles.list");
    }
  };

  const trace = state.trace.filter((entry) => `${entry.label} ${entry.summary} ${entry.detail ?? ""}`.toLowerCase().includes(traceFilter.toLowerCase()));
  const busy = Boolean(state.busyCommand);
  const connected = state.connection === "connected";
  const provisionResult = result?.cmd === "wifi.provision" || result?.code?.includes("PROVISION") ? {
    ok: result.ok,
    code: result.code,
    text: result.text,
    ssid: typeof result.data?.ssid === "string" ? result.data.ssid : ssid,
    ip: typeof result.data?.ip === "string" ? result.data.ip : undefined,
  } : undefined;
  const journeyStep = currentJourneyStep({ supportOk: support.supported, connected, networks, ssid, busyCommand: state.busyCommand, result: provisionResult });

  return (
    <Theme theme="white">
      <OperatorHeader support={support} state={state} targetPrefix={targetPrefix} busy={busy} onTargetPrefixChange={setTargetPrefix} />
      <Content className="app-content">
        <SupportNotice support={support} state={state} busy={busy} onConnect={connect} onDisconnect={disconnect} />
        <div className="operator-grid">
          <JourneyRail current={journeyStep} />
          <section className="workspace">
            <div className="tab-list" role="tablist" aria-label="工作区">
              <WorkspaceTab id="provision" active={activePanel} onSelect={setActivePanel}>配网</WorkspaceTab>
              <WorkspaceTab id="diagnostics" active={activePanel} onSelect={setActivePanel}>诊断</WorkspaceTab>
              <WorkspaceTab id="profiles" active={activePanel} onSelect={setActivePanel}>Wi-Fi 记忆</WorkspaceTab>
              <WorkspaceTab id="raw" active={activePanel} onSelect={setActivePanel}>Raw</WorkspaceTab>
            </div>
            {activePanel === "provision" && <ProvisionWorkbench connected={connected} busy={busy} deviceName={state.deviceName} networks={networks} ssid={ssid} password={password} networkFilter={networkFilter} result={provisionResult} error={error} onSsidChange={setSsid} onPasswordChange={setPassword} onNetworkFilterChange={setNetworkFilter} onScan={() => void runCommand("wifi.scan")} onProvision={() => window.confirm(`确认将设备 ${state.deviceName ?? ""} 连接到 Wi-Fi「${ssid}」？`) && void runCommand("wifi.provision", { ssid, pwd: password })} onResetResult={() => setResult(undefined)} />}
            {activePanel === "diagnostics" && <DiagnosticsPanel busy={busy} lastResponse={state.lastResponse} status={statusData} capabilities={capabilitiesData} heartbeat={heartbeatData} onLinkCheck={() => void runLinkCheck()} onStatus={() => void runCommand("system.status")} onCapabilities={() => void runCommand("system.capabilities")} onHeartbeat={() => void runCommand("link.heartbeat")} />}
            {activePanel === "profiles" && <ProfilesPanel busy={busy} profiles={profiles} selected={selectedProfiles} deleteResult={profileDeleteResult} onRefresh={() => void runCommand("wifi.profiles.list")} onToggle={(uuid) => setSelectedProfiles((items) => items.includes(uuid) ? items.filter((item) => item !== uuid) : [...items, uuid])} onDelete={() => window.confirm(`确认删除 ${selectedProfiles.length} 条非 active Wi-Fi 记忆？`) && void runCommand("wifi.profiles.delete", { uuids: selectedProfiles, force: false })} />}
            {activePanel === "raw" && <RawPanel busy={busy} command={rawCommand} args={rawArgs} onCommandChange={setRawCommand} onArgsChange={setRawArgs} onSend={() => sendRaw(rawCommand, rawArgs, runCommand, addTrace)} />}
          </section>
        </div>
        <DebugConsole debugMode={debugMode} traceFilter={traceFilter} tracePaused={tracePaused} expanded={traceExpanded} trace={trace} onDebugModeChange={setDebugMode} onTraceFilterChange={setTraceFilter} onTracePausedChange={setTracePaused} onExpandedChange={setTraceExpanded} onClear={() => setState((current) => ({ ...current, trace: [] }))} />
      </Content>
    </Theme>
  );
}

function WorkspaceTab({
  id,
  active,
  children,
  onSelect,
}: {
  id: PanelId;
  active: string;
  children: string;
  onSelect: (id: PanelId) => void;
}) {
  return <button className="tab-button" aria-selected={active === id} role="tab" type="button" onClick={() => onSelect(id)}>{children}</button>;
}

function sendRaw(command: GatewayCommand, args: string, run: (cmd: GatewayCommand, args?: JsonObject) => void, trace: (entry: TraceEntry) => void) {
  try {
    run(command, JSON.parse(args) as JsonObject);
  } catch {
    trace(traceEntry("ERR", "Raw args must be valid JSON object", undefined, "error"));
  }
}
