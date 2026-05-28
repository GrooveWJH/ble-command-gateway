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
  return "执行中";
}

export function commandLoadingLabel(command?: GatewayCommand): string {
  switch (command) {
    case "wifi.scan":
      return "正在扫描 Wi-Fi…";
    case "wifi.provision":
      return "正在下发配网…";
    case "system.status":
      return "正在刷新系统状态…";
    case "system.capabilities":
      return "正在读取协议能力…";
    case "link.heartbeat":
      return "正在检测链路心跳…";
    case "wifi.profiles.list":
      return "正在读取 Wi-Fi 管理信息…";
    case "wifi.profiles.delete":
      return "正在删除 Wi-Fi profile…";
    case "link.ack":
      return "正在确认链路…";
    default:
      return "空闲";
  }
}
