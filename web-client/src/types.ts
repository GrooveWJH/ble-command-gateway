export type JsonValue =
  | null
  | boolean
  | number
  | string
  | readonly JsonValue[]
  | { [key: string]: JsonValue };

export type JsonObject = { [key: string]: JsonValue };

export type GatewayCommand =
  | "link.heartbeat"
  | "system.status"
  | "system.capabilities"
  | "wifi.scan"
  | "wifi.provision"
  | "wifi.profiles.list"
  | "wifi.profiles.delete"
  | "link.ack";

export type ResponsePhase = "accepted" | "progress" | "result";

export interface CommandRequest {
  id: string;
  cmd: GatewayCommand;
  args: JsonObject;
  v: string;
}

export interface CommandResponse {
  id: string;
  cmd?: GatewayCommand;
  phase: ResponsePhase;
  seq: number;
  final: boolean;
  ok: boolean;
  code: string;
  text: string;
  data?: JsonObject;
  v: string;
}

export interface WifiNetwork {
  ssid: string;
  channel: string;
  signal: number;
}

export interface WifiProfile {
  uuid: string;
  name: string;
  ssid: string;
  active: boolean;
  device?: string;
  autoconnect: boolean;
}

export interface StatusInterfaceIpv4 {
  ifname: string;
  kind: "wifi" | "ethernet" | "other";
  ipv4: string;
}

export interface StatusResponseData {
  device_name: string;
  hostname: string;
  system: string;
  user: string;
  network?: string;
  ip?: string;
  interfaces: StatusInterfaceIpv4[];
}

export interface CapabilitiesResponseData {
  protocol_version: string;
  commands: string[];
  features: string[];
  payload_limit: number;
}

export interface HeartbeatResponseData {
  alive: boolean;
}

export interface WifiProfileDeleteItem {
  uuid: string;
  name: string;
  ssid: string;
}

export interface WifiProfileSkippedItem extends WifiProfileDeleteItem {
  reason: string;
}

export interface WifiProfileFailedItem extends WifiProfileDeleteItem {
  error: string;
}

export interface WifiProfilesDeleteResponseData {
  deleted: WifiProfileDeleteItem[];
  skipped: WifiProfileSkippedItem[];
  failed: WifiProfileFailedItem[];
}

export interface BrowserSupportState {
  supported: boolean;
  secureContext: boolean;
  hasBluetoothApi: boolean;
  userAgent: string;
  reason?: string;
}

export type DebugMode = "off" | "safe" | "unsafe";

export interface TraceEntry {
  id: string;
  at: string;
  level: "info" | "warn" | "error";
  label: string;
  summary: string;
  detail?: string;
}

export interface GatewayState {
  connection: "idle" | "connecting" | "connected" | "unsupported" | "error";
  deviceName?: string;
  busyCommand?: GatewayCommand;
  lastResponse?: CommandResponse;
  trace: TraceEntry[];
}

export type ProvisionStep = "connect" | "scan" | "credentials" | "provision" | "done";

export type ProvisionJourneyStep =
  | "environment"
  | "connect"
  | "scan"
  | "select"
  | "credentials"
  | "provision"
  | "result";

export type WorkspacePanel = "provision" | "diagnostics" | "profiles" | "raw";

export interface ProvisionResultView {
  ok: boolean;
  code: string;
  text: string;
  ssid?: string;
  ip?: string;
}

export interface ConnectionSummary {
  connected: boolean;
  statusLabel: string;
  deviceName: string;
  supportLabel: string;
}

export interface WifiSelection {
  ssid: string;
  passwordSet: boolean;
  hiddenNetwork: boolean;
}

export interface UserFacingError {
  title: string;
  subtitle: string;
  nextStep: string;
}
