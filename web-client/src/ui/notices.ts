import { preferredIp } from "./diagnostics";
import type { TFunction } from "../i18n/I18nProvider";
import type {
  AppNotice,
  AppNoticeKind,
  BasicInfoRefreshState,
  CommandResponse,
} from "../types";

export const NOTICE_CLOSE_MS = 240;

export function noticeTtl(kind: AppNoticeKind): number {
  return kind === "warning" || kind === "error" ? 7_000 : 4_000;
}

export function basicInfoNotice(refreshState: BasicInfoRefreshState, t: TFunction): Omit<AppNotice, "id"> | undefined {
  if (refreshState.state === "fresh") {
    return { kind: "success", title: t("notices.basicFreshTitle"), subtitle: t("notices.basicFreshSubtitle") };
  }
  if (refreshState.state !== "partial" && refreshState.state !== "failed") {
    return undefined;
  }
  const failed = refreshState.failures.map((item) => item.command).join(", ") || t("diagnostics.unknown");
  return {
    kind: refreshState.state === "partial" ? "warning" : "error",
    title: refreshState.state === "partial" ? t("notices.basicPartialTitle") : t("notices.basicFailedTitle"),
    subtitle: t("notices.basicFailureSubtitle", { commands: failed }),
  };
}

export function provisionNotice(
  response: CommandResponse,
  t: TFunction,
  refreshed?: CommandResponse,
): Omit<AppNotice, "id"> {
  if (!response.ok) {
    return { kind: "error", title: t("notices.provisionFailedTitle", { code: response.code }), subtitle: response.text };
  }
  if (refreshed?.ok) {
    return { kind: "success", title: t("notices.provisionSuccessTitle"), subtitle: t("notices.provisionSuccessSubtitle", { ip: preferredIp(refreshed) ?? t("diagnostics.noIp") }) };
  }
  return {
    kind: "warning",
    title: t("notices.provisionRefreshFailedTitle"),
    subtitle: t("notices.provisionRefreshFailedSubtitle"),
  };
}
