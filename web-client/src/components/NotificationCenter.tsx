import { ToastNotification } from "@carbon/react";

import type { AppNotice } from "../types";

export function NotificationCenter({
  notices,
  onDismiss,
}: {
  notices: AppNotice[];
  onDismiss: (id: string) => void;
}) {
  if (notices.length === 0) return null;
  return (
    <div aria-label="通知中心" className="yd-notification-center">
      {notices.map((notice) => (
        <div
          aria-label={`通知：${notice.title}`}
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
