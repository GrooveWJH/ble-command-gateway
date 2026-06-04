import { render, type RenderOptions } from "@testing-library/react";
import type { ReactElement } from "react";

import { I18nProvider, LANGUAGE_STORAGE_KEY, type Language } from "../i18n/I18nProvider";

export function renderWithI18n(ui: ReactElement, options?: RenderOptions & { language?: Language }) {
  if (options?.language) {
    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, options.language);
  }
  return render(<I18nProvider>{ui}</I18nProvider>, options);
}
