import { preferredIp } from "./diagnostics";
import type {
  AppNotice,
  AppNoticeKind,
  BasicInfoRefreshState,
  CommandResponse,
} from "../types";

export const NOTICE_CLOSE_MS = 240;

export function noticeTtl(kind: AppNoticeKind): number {
  return kind === "warning" || kind === "error" ? 7_000 : 4_000;
}

export function basicInfoNotice(refreshState: BasicInfoRefreshState): Omit<AppNotice, "id"> | undefined {
  if (refreshState.state === "fresh") {
    return { kind: "success", title: "基本信息已更新", subtitle: "系统状态与协议能力已读取完成。" };
  }
  if (refreshState.state !== "partial" && refreshState.state !== "failed") {
    return undefined;
  }
  const failed = refreshState.failures.map((item) => item.command).join("、") || "未知";
  return {
    kind: refreshState.state === "partial" ? "warning" : "error",
    title: refreshState.state === "partial" ? "基本信息部分读取成功" : "基本信息读取失败",
    subtitle: `失败命令：${failed}。已成功的数据会继续保留在页面上。`,
  };
}

export function provisionNotice(
  response: CommandResponse,
  refreshed?: CommandResponse,
): Omit<AppNotice, "id"> {
  if (!response.ok) {
    return { kind: "error", title: `配网失败：${response.code}`, subtitle: response.text };
  }
  if (refreshed?.ok) {
    return { kind: "success", title: "配网成功", subtitle: `已自动刷新系统状态。当前 IP：${preferredIp(refreshed) ?? "未获取"}` };
  }
  return {
    kind: "warning",
    title: "配网成功，但状态刷新失败",
    subtitle: "请保持被控端上电，并在基本信息页手动刷新确认最新 IP。",
  };
}
