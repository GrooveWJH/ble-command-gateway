import type { UserFacingError } from "../types";

export function explainError(message: string): UserFacingError {
  const lower = message.toLowerCase();
  if (lower.includes("web bluetooth") || lower.includes("secure context")) {
    return {
      title: "当前浏览器不可用",
      subtitle: "WebBluetooth 需要桌面 Chrome/Edge 或 Android Chrome，并通过 HTTPS 或 localhost 打开。",
      nextStep: "请更换浏览器或发布到 HTTPS 后再试。",
    };
  }
  if (lower.includes("device disconnected")) {
    return {
      title: "蓝牙连接已断开",
      subtitle: "设备可能离浏览器太远、被重启，或 BLE server 暂时不可用。",
      nextStep: "请靠近设备后重新连接，再执行扫描或配网。",
    };
  }
  if (lower.includes("not accepted") || lower.includes("timed out")) {
    return {
      title: "设备未及时响应",
      subtitle: "请求已经重试，但 server 没有在弱链路窗口内确认。",
      nextStep: "请确认设备上电、BLE 服务运行，并保持浏览器页面打开。",
    };
  }
  if (lower.includes("provision") || lower.includes("wifi")) {
    return {
      title: "配网失败",
      subtitle: "常见原因是密码错误、SSID 选错、频段不兼容或路由器信号过弱。",
      nextStep: "请重新扫描 Wi-Fi，确认 SSID 和密码后再次下发。",
    };
  }
  return {
    title: "命令执行失败",
    subtitle: message,
    nextStep: "请导出 trace 后交给开发者排查。",
  };
}
