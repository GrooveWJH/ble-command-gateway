import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { APP_VERSION } from "../appVersion";
import { OperatorHeader } from "./OperatorHeader";
import { renderWithI18n } from "../test/renderWithI18n";
import type { BrowserSupportState, GatewayState } from "../types";

const support: BrowserSupportState = {
  hasBluetoothApi: true,
  secureContext: true,
  supported: true,
  userAgent: "test",
};

const state: GatewayState = {
  connection: "idle",
  trace: [],
};

describe("OperatorHeader", () => {
  it("uses a single text-only language tile in English", () => {
    renderWithI18n(<OperatorHeader busy={false} state={state} support={support} targetPrefix="yundrone" />);

    const button = screen.getByRole("button", { name: "Switch language to Chinese" });
    expect(button).toHaveTextContent("EN");
    expect(button.querySelector("svg")).not.toBeInTheDocument();
    expect(screen.getByText(`Version: ${APP_VERSION}`)).toBeInTheDocument();
  });

  it("uses a single text-only language tile in Chinese", () => {
    renderWithI18n(<OperatorHeader busy={false} state={state} support={support} targetPrefix="yundrone" />, {
      language: "zh",
    });

    const button = screen.getByRole("button", { name: "切换到英文界面" });
    expect(button).toHaveTextContent("文");
    expect(button.querySelector("svg")).not.toBeInTheDocument();
    expect(screen.getByText(`版本：${APP_VERSION}`)).toBeInTheDocument();
  });
});
