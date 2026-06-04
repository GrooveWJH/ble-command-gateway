import { ToastNotification } from "@carbon/react";

import { useI18n } from "../i18n/useI18n";
import type { AppNotice } from "../types";

export function NotificationCenter({
  notices,
  onDismiss,
}: {
  notices: AppNotice[];
  onDismiss: (id: string) => void;
}) {
  const { t } = useI18n();
  if (notices.length === 0) return null;
  return (
    <div aria-label={t("notices.center")} className="yd-notification-center">
      {notices.map((notice) => (
        <div
          aria-label={t("notices.item", { title: notice.title })}
          className={`yd-notice ${notice.closing ? "yd-notice--closing" : ""}`}
          key={notice.id}
          role="status"
        >
          <ToastNotification
            caption=""
            hideCloseButton={false}
            kind={notice.kind}
            lowContrast
            onCloseButtonClick={() => onDismiss(notice.id)}
            subtitle={notice.subtitle}
            timeout={0}
            title={notice.title}
          />
        </div>
      ))}
    </div>
  );
}
