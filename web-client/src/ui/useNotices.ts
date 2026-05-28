import { useState } from "react";

import { NOTICE_CLOSE_MS, noticeTtl } from "./notices";
import type { AppNotice } from "../types";

export function useNotices() {
  const [notices, setNotices] = useState<AppNotice[]>([]);

  const removeNotice = (id: string) => {
    setNotices((current) => current.filter((notice) => notice.id !== id));
  };

  const dismissNotice = (id: string) => {
    setNotices((current) => current.map((notice) => (
      notice.id === id ? { ...notice, closing: true } : notice
    )));
    window.setTimeout(() => removeNotice(id), NOTICE_CLOSE_MS);
  };

  const notify = (notice: Omit<AppNotice, "id">) => {
    const id = crypto.randomUUID();
    const ttlMs = notice.ttlMs ?? noticeTtl(notice.kind);
    setNotices((current) => [...current.slice(-3), { ...notice, id, ttlMs }]);
    if (!notice.persistent) {
      window.setTimeout(() => dismissNotice(id), ttlMs);
    }
  };

  return { dismissNotice, notices, notify };
}
