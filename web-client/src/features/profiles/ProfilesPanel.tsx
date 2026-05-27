import { Button, Checkbox, Tag, Tile } from "@carbon/react";

import type { WifiProfile } from "../../types";

export function ProfilesPanel({
  busy,
  profiles,
  selected,
  onRefresh,
  onToggle,
  onDelete,
}: {
  busy: boolean;
  profiles: WifiProfile[];
  selected: string[];
  onRefresh: () => void;
  onToggle: (uuid: string) => void;
  onDelete: () => void;
}) {
  return (
    <section className="utility-panel">
      <div className="panel-heading">
        <div>
          <h2>Wi-Fi 记忆</h2>
          <p>读取并删除设备上保存的 NetworkManager Wi-Fi profile。</p>
        </div>
        <div className="button-row">
          <Button size="sm" disabled={busy} onClick={onRefresh}>刷新</Button>
          <Button size="sm" kind="danger" disabled={busy || selected.length === 0} onClick={onDelete}>
            删除选中
          </Button>
        </div>
      </div>
      <div className="profile-grid">
        {profiles.length === 0 ? (
          <Tile>尚未读取 profile。</Tile>
        ) : profiles.map((profile) => (
          <Tile key={profile.uuid} className="profile-card">
            <Checkbox
              id={`profile-${profile.uuid}`}
              labelText={profile.ssid}
              checked={selected.includes(profile.uuid)}
              onChange={() => onToggle(profile.uuid)}
            />
            <span className="mono">{profile.name} · {profile.uuid}</span>
            <Tag type={profile.active ? "green" : "gray"}>
              {profile.active ? "active" : "inactive"}
            </Tag>
          </Tile>
        ))}
      </div>
    </section>
  );
}
