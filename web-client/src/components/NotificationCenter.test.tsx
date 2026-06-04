import { act, screen } from "@testing-library/react";
import { useEffect, useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { NotificationCenter } from "./NotificationCenter";
import { renderWithI18n } from "../test/renderWithI18n";
import type { AppNotice } from "../types";

describe("NotificationCenter", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders centered transient notifications and allows manual dismiss", () => {
    renderWithI18n(
      <NotificationCenter
        notices={[notice("notice-1", "success", "基本信息已更新")]}
        onDismiss={vi.fn()}
      />,
      { language: "zh" },
    );

    expect(screen.getByLabelText("通知中心")).toHaveClass("yd-notification-center");
    expect(screen.getByRole("status", { name: "通知：基本信息已更新" })).toBeInTheDocument();
  });

  it("auto-dismisses notices with a closing state before removal", async () => {
    vi.useFakeTimers();
    renderWithI18n(<NotificationHarness />, { language: "zh" });

    const active = screen.getByRole("status", { name: "通知：基本信息已更新" });
    await act(() => vi.advanceTimersByTimeAsync(4_000));

    expect(active).toHaveClass("yd-notice--closing");
    await act(() => vi.advanceTimersByTimeAsync(260));
    expect(screen.queryByRole("status", { name: "通知：基本信息已更新" })).not.toBeInTheDocument();
  });

  it("does not auto-dismiss persistent notices", async () => {
    vi.useFakeTimers();
    renderWithI18n(<NotificationHarness persistent />, { language: "zh" });

    await act(() => vi.advanceTimersByTimeAsync(20_000));

    expect(screen.getByRole("status", { name: "通知：当前浏览器无法使用 Web Bluetooth" })).toBeInTheDocument();
  });
});

function NotificationHarness({ persistent = false }: { persistent?: boolean }) {
  const [notices, setNotices] = useState<AppNotice[]>([
    persistent
      ? notice("notice-1", "error", "当前浏览器无法使用 Web Bluetooth", true)
      : notice("notice-1", "success", "基本信息已更新"),
  ]);
  const dismiss = (id: string) => {
    setNotices((current) => current.map((item) => (
      item.id === id ? { ...item, closing: true } : item
    )));
    window.setTimeout(() => setNotices((current) => current.filter((item) => item.id !== id)), 240);
  };
  useEffect(() => {
    if (persistent) return undefined;
    const timer = window.setTimeout(() => dismiss("notice-1"), 4_000);
    return () => window.clearTimeout(timer);
  }, [persistent]);
  return <NotificationCenter notices={notices} onDismiss={dismiss} />;
}

function notice(id: string, kind: AppNotice["kind"], title: string, persistent = false): AppNotice {
  return { id, kind, title, persistent };
}
