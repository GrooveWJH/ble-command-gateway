import { createContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { messages, type MessageKey } from "./messages";

export type Language = "en" | "zh";
export type TFunction = (key: MessageKey, values?: Record<string, string | number>) => string;

export const LANGUAGE_STORAGE_KEY = "yundrone-web-lang";

export interface I18nContextValue {
  language: Language;
  setLanguage: (language: Language) => void;
  toggleLanguage: () => void;
  t: TFunction;
}

export const I18nContext = createContext<I18nContextValue | undefined>(undefined);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<Language>(() => readInitialLanguage());

  const setLanguage = (nextLanguage: Language) => {
    setLanguageState(nextLanguage);
  };

  useEffect(() => {
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
    document.title = messages[language]["app.documentTitle"];
    try {
      globalThis.localStorage?.setItem(LANGUAGE_STORAGE_KEY, language);
    } catch {
      // Storage is optional in embedded/test contexts.
    }
  }, [language]);

  const value = useMemo<I18nContextValue>(() => {
    const t: TFunction = (key, values) => interpolate(messages[language][key], values);
    return {
      language,
      setLanguage,
      toggleLanguage: () => setLanguage(language === "en" ? "zh" : "en"),
      t,
    };
  }, [language]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

function readInitialLanguage(): Language {
  try {
    return globalThis.localStorage?.getItem(LANGUAGE_STORAGE_KEY) === "zh" ? "zh" : "en";
  } catch {
    return "en";
  }
}

function interpolate(template: string, values?: Record<string, string | number>): string {
  if (!values) return template;
  return template.replace(/\{(\w+)\}/g, (match, key) => (
    Object.prototype.hasOwnProperty.call(values, key) ? String(values[key]) : match
  ));
}
