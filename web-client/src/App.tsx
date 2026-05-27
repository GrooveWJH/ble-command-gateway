import { Content, Tab, TabList, TabPanel, TabPanels, Tabs, Theme } from "@carbon/react";
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
  DebugMode,
  GatewayCommand,
  GatewayState,
  JsonObject,
  TraceEntry,
  UserFacingError,
  WifiNetwork,
  WifiProfile,
} from "./types";

interface AppProps {
  initialTrace?: TraceEntry[];
}

const DEFAULT_TRACE = [
  traceEntry("SYS", "Web console ready. 使用 Chrome/Edge 或 Android Chrome 连接设备。"),
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
  const [ssid, setSsid] = useState("");
  const [password, setPassword] = useState("");
  const [networkFilter, setNetworkFilter] = useState("");
  const [result, setResult] = useState<CommandResponse | undefined>();
  const [error, setError] = useState<UserFacingError | undefined>();
  const [selectedProfiles, setSelectedProfiles] = useState<string[]>([]);
  const [rawCommand, setRawCommand] = useState<GatewayCommand>("system.status");
  const [rawArgs, setRawArgs] = useState("{}");
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
    if (!support.supported) return;
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
    if (cmd === "wifi.provision" && !response.ok) setError(explainError(response.text));
    if (cmd === "wifi.profiles.list") {
      const loaded = Array.isArray(response.data?.profiles) ? response.data.profiles as unknown as WifiProfile[] : [];
      setProfiles(loaded);
    }
    if (cmd === "wifi.profiles.delete") {
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
          <Tabs>
            <TabList aria-label="工作区">
              <Tab>配网</Tab><Tab>诊断</Tab><Tab>Wi-Fi 记忆</Tab><Tab>Raw</Tab>
            </TabList>
            <TabPanels>
              <TabPanel><ProvisionWorkbench connected={connected} busy={busy} deviceName={state.deviceName} networks={networks} ssid={ssid} password={password} networkFilter={networkFilter} result={provisionResult} error={error} onSsidChange={setSsid} onPasswordChange={setPassword} onNetworkFilterChange={setNetworkFilter} onScan={() => void runCommand("wifi.scan")} onProvision={() => window.confirm(`确认将设备 ${state.deviceName ?? ""} 连接到 Wi-Fi「${ssid}」？`) && void runCommand("wifi.provision", { ssid, pwd: password })} onResetResult={() => setResult(undefined)} /></TabPanel>
              <TabPanel><DiagnosticsPanel busy={busy} lastResponse={state.lastResponse} onStatus={() => void runCommand("system.status")} onCapabilities={() => void runCommand("system.capabilities")} onHeartbeat={() => void runCommand("link.heartbeat")} /></TabPanel>
              <TabPanel><ProfilesPanel busy={busy} profiles={profiles} selected={selectedProfiles} onRefresh={() => void runCommand("wifi.profiles.list")} onToggle={(uuid) => setSelectedProfiles((items) => items.includes(uuid) ? items.filter((item) => item !== uuid) : [...items, uuid])} onDelete={() => window.confirm(`确认删除 ${selectedProfiles.length} 条 Wi-Fi 记忆？`) && void runCommand("wifi.profiles.delete", { uuids: selectedProfiles, force: true })} /></TabPanel>
              <TabPanel><RawPanel busy={busy} command={rawCommand} args={rawArgs} onCommandChange={setRawCommand} onArgsChange={setRawArgs} onSend={() => sendRaw(rawCommand, rawArgs, runCommand, addTrace)} /></TabPanel>
            </TabPanels>
          </Tabs>
        </div>
        <DebugConsole debugMode={debugMode} traceFilter={traceFilter} tracePaused={tracePaused} expanded={traceExpanded} trace={trace} onDebugModeChange={setDebugMode} onTraceFilterChange={setTraceFilter} onTracePausedChange={setTracePaused} onExpandedChange={setTraceExpanded} onClear={() => setState((current) => ({ ...current, trace: [] }))} />
      </Content>
    </Theme>
  );
}

function sendRaw(command: GatewayCommand, args: string, run: (cmd: GatewayCommand, args?: JsonObject) => void, trace: (entry: TraceEntry) => void) {
  try {
    run(command, JSON.parse(args) as JsonObject);
  } catch {
    trace(traceEntry("ERR", "Raw args must be valid JSON object", undefined, "error"));
  }
}
