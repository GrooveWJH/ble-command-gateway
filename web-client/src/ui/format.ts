import type { GatewayCommand, GatewayState } from "../types";
import type { TFunction } from "../i18n/I18nProvider";

export function connectionLabel(connection: GatewayState["connection"], t: TFunction): string {
  switch (connection) {
    case "connected":
      return t("connection.connected");
    case "connecting":
      return t("connection.connecting");
    case "unsupported":
      return t("connection.unsupported");
    case "error":
      return t("connection.error");
    case "idle":
      return t("connection.idle");
  }
}

export function busyLabel(command: GatewayCommand | undefined, t: TFunction): string {
  if (!command) {
    return t("busy.idle");
  }
  return t("busy.running");
}

export function commandLoadingLabel(command: GatewayCommand | undefined, t: TFunction): string {
  switch (command) {
    case "wifi.scan":
      return t("loading.wifi.scan");
    case "wifi.provision":
      return t("loading.wifi.provision");
    case "system.status":
      return t("loading.system.status");
    case "system.capabilities":
      return t("loading.system.capabilities");
    case "link.heartbeat":
      return t("loading.link.heartbeat");
    case "wifi.profiles.list":
      return t("loading.wifi.profiles.list");
    case "wifi.profiles.delete":
      return t("loading.wifi.profiles.delete");
    case "link.ack":
      return t("loading.default");
    default:
      return t("busy.idle");
  }
}
