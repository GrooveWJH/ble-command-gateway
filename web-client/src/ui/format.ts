import type { GatewayCommand, GatewayState } from "../types";

export function connectionLabel(connection: GatewayState["connection"]): string {
  switch (connection) {
    case "connected":
      return "已连接";
    case "connecting":
      return "连接中";
    case "unsupported":
      return "不可用";
    case "error":
      return "连接错误";
    case "idle":
      return "未连接";
  }
}

export function busyLabel(command?: GatewayCommand): string {
  if (!command) {
    return "空闲";
  }
  return command;
}
