import { InlineLoading } from "@carbon/react";

import { useI18n } from "../i18n/useI18n";

export function BleCommandOverlay({ active, label }: { active: boolean; label: string }) {
  const { t } = useI18n();
  if (!active) return null;
  return (
    <div className="yd-ble-overlay" data-testid="ble-command-overlay">
      <div
        aria-busy="true"
        aria-label={t("overlay.aria")}
        aria-live="polite"
        className="yd-ble-overlay__card"
        role="status"
      >
        <InlineLoading
          description={label}
          iconDescription={t("overlay.processing")}
          status="active"
        />
        <p>{t("overlay.keepNear")}</p>
      </div>
    </div>
  );
}
