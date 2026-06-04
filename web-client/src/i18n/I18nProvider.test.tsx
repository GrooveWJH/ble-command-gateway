import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { I18nProvider, LANGUAGE_STORAGE_KEY } from "./I18nProvider";
import { enMessages } from "./messages.en";
import { zhMessages } from "./messages.zh";
import { useI18n } from "./useI18n";

describe("I18nProvider", () => {
  it("defaults to English and persists Chinese after switching", () => {
    render(
      <I18nProvider>
        <LanguageProbe />
      </I18nProvider>,
    );

    expect(screen.getByText("YunDrone BLE Wi-Fi Tool")).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("en");

    fireEvent.click(screen.getByRole("button", { name: "Switch language to Chinese" }));

    expect(screen.getByText("YunDrone 配网工作台")).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("zh-CN");
    expect(window.localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("zh");
  });

  it("restores the persisted Chinese language on the next visit", () => {
    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, "zh");

    render(
      <I18nProvider>
        <LanguageProbe />
      </I18nProvider>,
    );

    expect(screen.getByText("YunDrone 配网工作台")).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("zh-CN");
  });

  it("keeps Chinese and English message keys in lockstep", () => {
    expect(Object.keys(zhMessages).sort()).toEqual(Object.keys(enMessages).sort());
  });
});

function LanguageProbe() {
  const { language, setLanguage, t } = useI18n();
  const nextLanguage = language === "en" ? "zh" : "en";
  return (
    <>
      <h1>{t("app.productName")}</h1>
      <button type="button" onClick={() => setLanguage(nextLanguage)}>
        {t("header.languageSwitchLabel")}
      </button>
    </>
  );
}
