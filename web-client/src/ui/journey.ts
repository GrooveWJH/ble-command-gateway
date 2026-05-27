import type {
  GatewayCommand,
  ProvisionJourneyStep,
  ProvisionResultView,
  WifiNetwork,
} from "../types";

export const JOURNEY_STEPS: Array<{ id: ProvisionJourneyStep; label: string }> = [
  { id: "environment", label: "环境检查" },
  { id: "connect", label: "连接设备" },
  { id: "scan", label: "扫描 Wi-Fi" },
  { id: "select", label: "选择网络" },
  { id: "credentials", label: "输入密码" },
  { id: "provision", label: "下发配网" },
  { id: "result", label: "结果确认" },
];

export function currentJourneyStep(input: {
  supportOk: boolean;
  connected: boolean;
  networks: WifiNetwork[];
  ssid: string;
  busyCommand?: GatewayCommand;
  result?: ProvisionResultView;
}): ProvisionJourneyStep {
  if (!input.supportOk) {
    return "environment";
  }
  if (input.result) {
    return "result";
  }
  if (input.busyCommand === "wifi.provision") {
    return "provision";
  }
  if (!input.connected) {
    return "connect";
  }
  if (input.busyCommand === "wifi.scan" || input.networks.length === 0) {
    return "scan";
  }
  if (!input.ssid.trim()) {
    return "select";
  }
  return "credentials";
}

export function journeyIndex(step: ProvisionJourneyStep): number {
  return Math.max(
    0,
    JOURNEY_STEPS.findIndex((item) => item.id === step),
  );
}
