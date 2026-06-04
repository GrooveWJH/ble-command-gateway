import type { UserFacingError } from "../types";
import type { TFunction } from "../i18n/I18nProvider";

export function explainError(message: string, t: TFunction): UserFacingError {
  const lower = message.toLowerCase();
  if (lower.includes("web bluetooth") || lower.includes("secure context")) {
    return {
      title: t("errors.browserTitle"),
      subtitle: t("errors.browserSubtitle"),
      nextStep: t("errors.browserNext"),
    };
  }
  if (lower.includes("device disconnected")) {
    return {
      title: t("errors.disconnectedTitle"),
      subtitle: t("errors.disconnectedSubtitle"),
      nextStep: t("errors.disconnectedNext"),
    };
  }
  if (lower.includes("not accepted") || lower.includes("timed out")) {
    return {
      title: t("errors.timeoutTitle"),
      subtitle: t("errors.timeoutSubtitle"),
      nextStep: t("errors.timeoutNext"),
    };
  }
  if (lower.includes("provision") || lower.includes("wifi")) {
    return {
      title: t("errors.provisionTitle"),
      subtitle: t("errors.provisionSubtitle"),
      nextStep: t("errors.provisionNext"),
    };
  }
  return {
    title: t("errors.commandTitle"),
    subtitle: message,
    nextStep: t("errors.commandNext"),
  };
}
