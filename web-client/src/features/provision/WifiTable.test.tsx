import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { WifiTable } from "./WifiTable";

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
