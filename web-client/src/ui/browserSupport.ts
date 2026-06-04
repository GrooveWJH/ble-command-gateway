import type { TFunction } from "../i18n/I18nProvider";
import type { BrowserSupportState } from "../types";

export function browserSupportReason(support: BrowserSupportState, t: TFunction): string {
  if (support.reasonKey) {
    return t(support.reasonKey);
  }
  return support.reason ?? t("app.unsupportedReason");
}
