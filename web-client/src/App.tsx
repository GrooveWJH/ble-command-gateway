import { Content, Tab, TabList, TabPanel, TabPanels, Tabs, Theme } from "@carbon/react";
import { useEffect, useMemo, useRef, useState } from "react";

import { getBrowserSupport, requestYundroneDevice } from "./ble/webBluetooth";
import { BleCommandOverlay } from "./components/BleCommandOverlay";
import { NotificationCenter } from "./components/NotificationCenter";
import { OperatorHeader } from "./components/OperatorHeader";
import { SupportNotice } from "./components/SupportNotice";
import { BasicInfoPanel } from "./features/basic/BasicInfoPanel";
import { basicInfoStateFromResponses } from "./features/basic/refresh";
import { DebugDock } from "./features/debug/DebugDock";
import { ProfilesPanel } from "./features/profiles/ProfilesPanel";
import { ProvisionWorkbench } from "./features/provision/ProvisionWorkbench";
import { RawPanel } from "./features/raw/RawPanel";
import { GatewayClient } from "./services/GatewayClient";
import { traceEntry } from "./services/trace";
import { explainError } from "./ui/errors";
import { commandLoadingLabel } from "./ui/format";
import { currentJourneyStep } from "./ui/journey";
import { basicInfoNotice, provisionNotice } from "./ui/notices";
import { provisionResultView } from "./ui/provisionResult";
import { sendRawCommand } from "./ui/rawCommand";
import { safeStorageGet, safeStorageSet } from "./ui/storage";
import { useNotices } from "./ui/useNotices";
import type { CommandResponse, DebugMode, GatewayCommand, GatewayState, JsonObject, TraceEntry, UserFacingError, WifiNetwork, WifiProfile } from "./types";
import { WORKSPACE_TABS, workspaceTabAt, workspaceTabIndex, type WorkspaceTab } from "./workspaceTabs";

interface AppProps { initialTrace?: TraceEntry[]; }

const DEFAULT_TRACE = [traceEntry("SYS", "Web console ready. 使用 Chrome/Edge 或 Android Chrome 连接设备。")];

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
  const [provisionResponse, setProvisionResponse] = useState<CommandResponse | undefined>();
  const [statusResponse, setStatusResponse] = useState<CommandResponse | undefined>();
  const [capabilitiesResponse, setCapabilitiesResponse] = useState<CommandResponse | undefined>();
  const [heartbeatResponse, setHeartbeatResponse] = useState<CommandResponse | undefined>();
  const [provisionRefresh, setProvisionRefresh] = useState<"refreshing" | "fresh" | "failed" | undefined>();
  const [error, setError] = useState<UserFacingError | undefined>();
  const [selectedProfiles, setSelectedProfiles] = useState<string[]>([]);
  const [rawCommand, setRawCommand] = useState<GatewayCommand>("system.status");
  const [rawArgs, setRawArgs] = useState("{}");
  const [traceFilter, setTraceFilter] = useState("");
  const [tracePaused, setTracePaused] = useState(false);
  const [traceExpanded, setTraceExpanded] = useState(false);
  const [activeTab, setActiveTab] = useState<WorkspaceTab>("basic");
  const { dismissNotice, notices, notify } = useNotices();

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
      const userError = explainError(message);
      notify({
        kind: "error",
        persistent: true,
        title: userError.title,
        subtitle: `${userError.subtitle} ${userError.nextStep}`,
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
      notify({ kind: "success", title: "设备已连接", subtitle: connection.device.name ?? "YunDrone BLE" });
      setActiveTab("basic");
      void refreshBasicInfoAfterConnect();
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setError(explainError(message));
      notify({ kind: "error", title: "连接失败", subtitle: message });
      setState((current) => ({ ...current, connection: "error" }));
      addTrace(traceEntry("ERR", message, undefined, "error"));
    }
  };

  const handleDisconnected = () => {
    clientRef.current = null;
    setError(explainError("Device disconnected"));
    notify({ kind: "warning", title: "设备已断开", subtitle: "连接已关闭，需要继续操作时请重新连接被控端。" });
    setState((current) => ({ ...current, connection: support.supported ? "idle" : "unsupported", deviceName: undefined, busyCommand: undefined }));
  };

  const disconnect = () => {
    clientRef.current?.disconnect();
    handleDisconnected();
    addTrace(traceEntry("SYS", "Disconnected"));
  };

  const refreshBasicInfoAfterConnect = async () => {
    addTrace(traceEntry("SYS", "basic-info:auto status start"));
    const status = await executeCommand("system.status");
    addTrace(traceEntry("SYS", "basic-info:auto status done"));
    addTrace(traceEntry("SYS", "basic-info:auto capabilities start"));
    const capabilities = await executeCommand("system.capabilities");
    addTrace(traceEntry("SYS", "basic-info:auto capabilities done"));
    const refreshState = basicInfoStateFromResponses(status, capabilities);
    const notice = basicInfoNotice(refreshState);
    if (notice) notify(notice);
  };

  const executeCommand = async (cmd: GatewayCommand, args: JsonObject = {}) => {
    if (!clientRef.current) {
      const message = "请先点击连接设备，在浏览器蓝牙选择器里选择 yundrone-* 设备。";
      setError(explainError(message));
      notify({ kind: "warning", title: "尚未连接设备", subtitle: message });
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
      notify({ kind: "error", title: "命令执行失败", subtitle: message });
      addTrace(traceEntry("ERR", message, undefined, "error"));
      return undefined;
    } finally {
      setState((current) => ({ ...current, busyCommand: undefined }));
    }
  };

  const runCommand = async (cmd: GatewayCommand, args: JsonObject = {}) => {
    const response = await executeCommand(cmd, args);
    if (cmd === "wifi.provision" && response?.ok) {
      setProvisionRefresh("refreshing");
      const refreshed = await executeCommand("system.status");
      setProvisionRefresh(refreshed?.ok ? "fresh" : "failed");
      notify(provisionNotice(response, refreshed));
      setActiveTab("basic");
    } else if (cmd === "wifi.provision" && response) {
      notify(provisionNotice(response));
    }
    return response;
  };

  const applyResponse = (cmd: GatewayCommand, response: CommandResponse) => {
    if (cmd === "system.status") setStatusResponse(response);
    if (cmd === "system.capabilities") setCapabilitiesResponse(response);
    if (cmd === "link.heartbeat") setHeartbeatResponse(response);
    if (cmd === "wifi.scan") {
      const loaded = Array.isArray(response.data?.networks) ? response.data.networks as unknown as WifiNetwork[] : [];
      setNetworks([...loaded].sort((left, right) => right.signal - left.signal));
      setNetworkFilter("");
    }
    if (cmd === "wifi.provision") {
      setProvisionResponse(response);
      setProvisionRefresh(undefined);
      if (!response.ok) setError(explainError(response.text));
    }
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
  const overlayActive = state.connection === "connecting" || busy;
  const overlayLabel = state.connection === "connecting"
    ? "正在连接被控端…"
    : commandLoadingLabel(state.busyCommand);
  const provisionResult = provisionResultView({
    passwordlessSsid: ssid,
    provisionRefresh,
    provisionResponse,
    statusResponse,
  });
  const journeyStep = currentJourneyStep({ supportOk: support.supported, connected, networks, ssid, busyCommand: state.busyCommand, result: provisionResult });
  const selectedIndex = workspaceTabIndex(activeTab);

  return (
    <Theme theme="white">
      <OperatorHeader support={support} state={state} targetPrefix={targetPrefix} busy={busy} />
      <Content className="yd-app-content">
        <SupportNotice support={support} state={state} busy={busy} targetPrefix={targetPrefix} onTargetPrefixChange={setTargetPrefix} onConnect={connect} onDisconnect={disconnect} />
        <div className="yd-workspace-shell">
          <div className="yd-tabs-frame">
            <Tabs
              selectedIndex={selectedIndex}
              onChange={({ selectedIndex: nextIndex }) => setActiveTab(workspaceTabAt(nextIndex))}
            >
              <TabList aria-label="工作区">
                {WORKSPACE_TABS.map((tab) => <Tab key={tab.key}>{tab.label}</Tab>)}
              </TabList>
              <TabPanels>
                <TabPanel><BasicInfoPanel busyCommand={state.busyCommand} statusResponse={statusResponse} capabilitiesResponse={capabilitiesResponse} heartbeatResponse={heartbeatResponse} onStatus={() => void runCommand("system.status")} onCapabilities={() => void runCommand("system.capabilities")} onHeartbeat={() => void runCommand("link.heartbeat")} /></TabPanel>
                <TabPanel><ProvisionWorkbench connected={connected} busyCommand={state.busyCommand} deviceName={state.deviceName} journeyStep={journeyStep} networks={networks} ssid={ssid} password={password} networkFilter={networkFilter} result={provisionResult} error={error} onSsidChange={setSsid} onPasswordChange={setPassword} onNetworkFilterChange={setNetworkFilter} onScan={() => void runCommand("wifi.scan")} onProvision={() => window.confirm(`确认将设备 ${state.deviceName ?? ""} 连接到 Wi-Fi「${ssid}」？`) && void runCommand("wifi.provision", { ssid, pwd: password })} onResetResult={() => setProvisionResponse(undefined)} /></TabPanel>
                <TabPanel><ProfilesPanel busyCommand={state.busyCommand} profiles={profiles} selected={selectedProfiles} onRefresh={() => void runCommand("wifi.profiles.list")} onToggle={(uuid) => setSelectedProfiles((items) => items.includes(uuid) ? items.filter((item) => item !== uuid) : [...items, uuid])} onDelete={() => window.confirm(`确认删除 ${selectedProfiles.length} 条 Wi-Fi profile？`) && void runCommand("wifi.profiles.delete", { uuids: selectedProfiles, force: true })} /></TabPanel>
                <TabPanel><RawPanel busyCommand={state.busyCommand} command={rawCommand} args={rawArgs} onCommandChange={setRawCommand} onArgsChange={setRawArgs} onSend={() => sendRawCommand(rawCommand, rawArgs, runCommand, addTrace)} /></TabPanel>
              </TabPanels>
            </Tabs>
          </div>
        </div>
        <NotificationCenter notices={notices} onDismiss={dismissNotice} />
        <BleCommandOverlay active={overlayActive} label={overlayLabel} />
        <DebugDock debugMode={debugMode} traceFilter={traceFilter} tracePaused={tracePaused} expanded={traceExpanded} trace={trace} onDebugModeChange={setDebugMode} onTraceFilterChange={setTraceFilter} onTracePausedChange={setTracePaused} onExpandedChange={setTraceExpanded} onClear={() => setState((current) => ({ ...current, trace: [] }))} />
      </Content>
    </Theme>
  );
}
