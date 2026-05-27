import { Button, Checkbox, InlineNotification, Tag, Tile } from "@carbon/react";

import type { WifiProfile, WifiProfilesDeleteResponseData } from "../../types";

export function ProfilesPanel({
  busy,
  profiles,
  selected,
  deleteResult,
  onRefresh,
  onToggle,
  onDelete,
}: {
  busy: boolean;
  profiles: WifiProfile[];
  selected: string[];
  deleteResult?: WifiProfilesDeleteResponseData;
  onRefresh: () => void;
  onToggle: (uuid: string) => void;
  onDelete: () => void;
}) {
  const deletable = new Set(profiles.filter((profile) => !profile.active).map((profile) => profile.uuid));
  return (
    <section className="utility-panel">
      <div className="panel-heading">
        <div>
          <h2>Wi-Fi 记忆</h2>
          <p>读取并删除设备上保存的 NetworkManager Wi-Fi profile。默认保护 active profile。</p>
        </div>
        <div className="button-row">
          <Button disabled={busy} onClick={onRefresh}>刷新</Button>
          <Button kind="danger" disabled={busy || selected.length === 0} onClick={onDelete}>
            删除选中
          </Button>
        </div>
      </div>
      {deleteResult && <DeleteResultNotice result={deleteResult} />}
      <div className="profile-grid">
        {profiles.length === 0 ? (
          <Tile className="summary-tile">尚未读取 profile。</Tile>
        ) : profiles.map((profile) => (
          <Tile key={profile.uuid} className="profile-card">
            <Checkbox
              id={`profile-${profile.uuid}`}
              labelText={profile.ssid}
              checked={selected.includes(profile.uuid)}
              disabled={!deletable.has(profile.uuid) || busy}
              onChange={() => onToggle(profile.uuid)}
            />
            <span className="mono">{profile.name} · {profile.uuid}</span>
            <Tag type={profile.active ? "green" : "gray"}>
              {profile.active ? "active" : "inactive"}
            </Tag>
            <span>{profile.device ? `设备：${profile.device}` : "未绑定设备"} · autoconnect: {profile.autoconnect ? "yes" : "no"}</span>
          </Tile>
        ))}
      </div>
    </section>
  );
}

function DeleteResultNotice({ result }: { result: WifiProfilesDeleteResponseData }) {
  const deleted = result.deleted ?? [];
  const skipped = result.skipped ?? [];
  const failed = result.failed ?? [];
  return (
    <InlineNotification
      kind={failed.length > 0 ? "error" : "success"}
      lowContrast
      title={`删除完成：deleted=${deleted.length}, skipped=${skipped.length}, failed=${failed.length}`}
      subtitle={formatDeleteDetails(result)}
    />
  );
}

function formatDeleteDetails(result: WifiProfilesDeleteResponseData): string {
  const skipped = (result.skipped ?? []).map((item) => `${item.ssid}: ${item.reason}`);
  const failed = (result.failed ?? []).map((item) => `${item.ssid}: ${item.error}`);
  return [...skipped, ...failed].join("；") || "已按 CLI 默认策略删除非 active profile。";
}
