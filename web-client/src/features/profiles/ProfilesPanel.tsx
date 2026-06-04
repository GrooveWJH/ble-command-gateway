import { Button, Checkbox, Tag, Tile } from "@carbon/react";

import { useI18n } from "../../i18n/useI18n";
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
  const { t } = useI18n();
  const busy = Boolean(busyCommand);
  return (
    <section className="yd-utility-panel">
      <div className="yd-panel-heading">
        <div>
          <h2>{t("profiles.title")}</h2>
          <p>{t("profiles.subtitle")}</p>
        </div>
        <div className="yd-button-row">
          <Button size="sm" disabled={busy} onClick={onRefresh}>{t("profiles.refresh")}</Button>
          <Button size="sm" kind="danger" disabled={busy || selected.length === 0} onClick={onDelete}>
            {t("profiles.deleteSelected")}
          </Button>
        </div>
      </div>
      <div className="yd-profile-grid">
        {profiles.length === 0 ? (
          <Tile>{t("profiles.empty")}</Tile>
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
              {profile.active ? t("profiles.active") : t("profiles.inactive")}
            </Tag>
          </Tile>
        ))}
      </div>
    </section>
  );
}
