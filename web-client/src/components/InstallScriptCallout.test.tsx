import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { InstallScriptCallout, INSTALL_COMMAND } from "./InstallScriptCallout";
import { renderWithI18n } from "../test/renderWithI18n";

describe("InstallScriptCallout", () => {
  it("opens a glowing one-click deployment modal and copies the command", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    renderWithI18n(<InstallScriptCallout />, { language: "zh" });

    const trigger = screen.getByRole("button", { name: "一键部署工具" });
    expect(trigger).toHaveClass("yd-install-badge");

    fireEvent.click(trigger);

    expect(await screen.findByRole("heading", { name: "一键部署工具" })).toBeInTheDocument();
    expect(screen.getByText(INSTALL_COMMAND)).toBeInTheDocument();
    expect(screen.getByText(/Gum TUI 终端界面/)).toBeInTheDocument();
    expect(screen.getByText(/Linux AMD64 \/ ARM64/)).toBeInTheDocument();
    expect(screen.getByText(/macOS 支持 TUI Client/)).toBeInTheDocument();
    expect(screen.getByText(/本网站会连接这个 yundrone-\* 设备/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "复制一键部署指令" }));

    await waitFor(() => expect(writeText).toHaveBeenCalledWith(INSTALL_COMMAND));
  });
});
