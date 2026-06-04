import type {
  GatewayCommand,
  ProvisionJourneyStep,
  ProvisionResultView,
  WifiNetwork,
} from "../types";
import type { MessageKey } from "../i18n/messages";

export const JOURNEY_STEPS: Array<{ id: ProvisionJourneyStep; labelKey: MessageKey }> = [
  { id: "environment", labelKey: "steps.environmentLabel" },
  { id: "connect", labelKey: "steps.connectLabel" },
  { id: "scan", labelKey: "steps.scanLabel" },
  { id: "select", labelKey: "steps.selectLabel" },
  { id: "credentials", labelKey: "steps.credentialsLabel" },
  { id: "provision", labelKey: "steps.provisionLabel" },
  { id: "result", labelKey: "steps.resultLabel" },
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
