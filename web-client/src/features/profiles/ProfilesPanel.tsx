import { Button, Checkbox, Tag, Tile } from "@carbon/react";

import type { GatewayCommand, WifiProfile } from "../../types";

export function ProfilesPanel({
  busyCommand,
  profiles,
  selected,
  onRefresh,
  onToggle,
  onDelete,
}: {
  busyCommand?: GatewayCommand;
  profiles: WifiProfile[];
  selected: string[];
  onRefresh: () => void;
  onToggle: (uuid: string) => void;
  onDelete: () => void;
}) {
  const busy = Boolean(busyCommand);
  return (
    <section className="yd-utility-panel">
      <div className="yd-panel-heading">
        <div>
          <h2>Wi-Fi 管理</h2>
          <p>读取并删除被控端保存的 NetworkManager Wi-Fi profile。</p>
        </div>
        <div className="yd-button-row">
          <Button size="sm" disabled={busy} onClick={onRefresh}>刷新</Button>
          <Button size="sm" kind="danger" disabled={busy || selected.length === 0} onClick={onDelete}>
            删除选中
          </Button>
        </div>
      </div>
      <div className="yd-profile-grid">
        {profiles.length === 0 ? (
          <Tile>尚未读取 Wi-Fi profile。</Tile>
        ) : profiles.map((profile) => (
          <Tile key={profile.uuid} className="yd-profile-card">
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
