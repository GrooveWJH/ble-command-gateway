export const WORKSPACE_TABS = [
  { key: "basic", label: "基本信息" },
  { key: "provision", label: "Wi-Fi 配网" },
  { key: "profiles", label: "Wi-Fi 管理" },
  { key: "advanced", label: "高级命令" },
] as const;

export type WorkspaceTab = typeof WORKSPACE_TABS[number]["key"];

export function workspaceTabIndex(tab: WorkspaceTab): number {
  return Math.max(0, WORKSPACE_TABS.findIndex((item) => item.key === tab));
}

export function workspaceTabAt(index: number): WorkspaceTab {
  return WORKSPACE_TABS[index]?.key ?? "basic";
}
