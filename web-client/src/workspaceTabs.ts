export const WORKSPACE_TABS = [
  { key: "basic", labelKey: "tabs.basic" },
  { key: "provision", labelKey: "tabs.provision" },
  { key: "profiles", labelKey: "tabs.profiles" },
  { key: "advanced", labelKey: "tabs.advanced" },
] as const;

export type WorkspaceTab = typeof WORKSPACE_TABS[number]["key"];

export function workspaceTabIndex(tab: WorkspaceTab): number {
  return Math.max(0, WORKSPACE_TABS.findIndex((item) => item.key === tab));
}

export function workspaceTabAt(index: number): WorkspaceTab {
  return WORKSPACE_TABS[index]?.key ?? "basic";
}
