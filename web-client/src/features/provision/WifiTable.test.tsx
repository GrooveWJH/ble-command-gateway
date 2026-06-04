import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { WifiTable } from "./WifiTable";
import { messages } from "../../i18n/messages";

const t = (key: keyof typeof messages.zh, values?: Record<string, string | number>) => {
  let message = messages.zh[key];
  if (!values) return message;
  for (const [name, value] of Object.entries(values)) {
    message = message.replace(`{${name}}`, String(value));
  }
  return message;
};

describe("WifiTable", () => {
  it("shows color-coded signal quality for RSSI and percent values", () => {
    render(
      <WifiTable
        connected
        filter=""
        networks={[
          { ssid: "strong-rssi", signal: -52, channel: "6" },
          { ssid: "mid-rssi", signal: -68, channel: "11" },
          { ssid: "weak-rssi", signal: -83, channel: "1" },
          { ssid: "strong-percent", signal: 82, channel: "36" },
        ]}
        t={t}
        onFilterChange={vi.fn()}
        onSelect={vi.fn()}
      />,
    );

    expect(screen.getByLabelText("信号强：-52")).toHaveClass("yd-signal-quality--strong");
    expect(screen.getByLabelText("信号中：-68")).toHaveClass("yd-signal-quality--medium");
    expect(screen.getByLabelText("信号弱：-83")).toHaveClass("yd-signal-quality--weak");
    expect(screen.getByLabelText("信号强：82")).toHaveClass("yd-signal-quality--strong");
  });

  it("keeps row selection behavior when signal cells are customized", () => {
    const onSelect = vi.fn();
    render(
      <WifiTable
        connected
        filter=""
        networks={[{ ssid: "LabWiFi", signal: -52, channel: "6" }]}
        t={t}
        onFilterChange={vi.fn()}
        onSelect={onSelect}
      />,
    );

    fireEvent.click(screen.getByText("LabWiFi"));

    expect(onSelect).toHaveBeenCalledWith("LabWiFi");
  });

  it("merges duplicate SSIDs and keeps the best signal plus all channels", () => {
    const onSelect = vi.fn();
    render(
      <WifiTable
        connected
        filter=""
        networks={[
          { ssid: "FactoryWiFi", signal: 62, channel: "1" },
          { ssid: "FactoryWiFi", signal: 88, channel: "11" },
          { ssid: "GuestWiFi", signal: 44, channel: "6" },
          { ssid: "FactoryWiFi", signal: 75, channel: "6" },
        ]}
        t={t}
        onFilterChange={vi.fn()}
        onSelect={onSelect}
      />,
    );

    expect(screen.getAllByText("FactoryWiFi")).toHaveLength(1);
    expect(screen.getByLabelText("信号强：88")).toBeInTheDocument();
    expect(screen.getByText("1 / 6 / 11")).toBeInTheDocument();
    expect(screen.getByText("3 个接入点")).toBeInTheDocument();
    expect(screen.getByText(/发现 2 个候选网络/)).toBeInTheDocument();

    fireEvent.click(screen.getByText("FactoryWiFi"));

    expect(onSelect).toHaveBeenCalledWith("FactoryWiFi");
  });
});
