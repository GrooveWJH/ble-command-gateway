import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import { bluetoothConnection, capabilitiesResponse, provisionResponse, statusResponse } from "./test/fixtures";
import type { BrowserSupportState, CommandResponse } from "./types";

const TEST_USER_AGENT = "Mozilla/5.0 Chrome/120.0 Test";

const mocks = vi.hoisted(() => ({
  disconnect: vi.fn(),
  requestDevice: vi.fn(),
  runCommand: vi.fn(),
  support: unsupportedState(),
}));

vi.mock("./ble/webBluetooth", () => ({
  getBrowserSupport: () => mocks.support,
  requestYundroneDevice: mocks.requestDevice,
}));

vi.mock("./services/GatewayClient", () => ({
  GatewayClient: vi.fn().mockImplementation(function GatewayClientMock() {
    return {
      runCommand: mocks.runCommand,
      disconnect: mocks.disconnect,
      setDebugMode: vi.fn(),
    };
  }),
}));

describe("App notifications", () => {
  beforeEach(() => {
    mocks.support = {
      supported: true,
      secureContext: true,
      hasBluetoothApi: true,
      userAgent: TEST_USER_AGENT,
    };
    mocks.requestDevice.mockReset();
    mocks.requestDevice.mockResolvedValue(bluetoothConnection());
    mocks.runCommand.mockReset();
    mocks.disconnect.mockReset();
    vi.spyOn(window, "confirm").mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    window.localStorage.clear();
  });

  it("shows successful basic info refresh in the top notification center", async () => {
    mocks.runCommand
      .mockResolvedValueOnce(statusResponse())
      .mockResolvedValueOnce({ ...capabilitiesResponse(), ok: false } satisfies CommandResponse);
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "连接设备" }));

    expect(await screen.findByRole("status", { name: "通知：基本信息已更新" })).toBeInTheDocument();
    expect(screen.queryByText("基本信息部分读取成功")).not.toBeInTheDocument();
    expect(screen.queryByText("基本信息读取失败")).not.toBeInTheDocument();
  });

  it("shows provisioning results in the top notification center", async () => {
    mocks.runCommand
      .mockResolvedValueOnce(statusResponse())
      .mockResolvedValueOnce(capabilitiesResponse())
      .mockResolvedValueOnce(provisionResponse())
      .mockResolvedValueOnce(statusResponse());
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "连接设备" }));
    await waitFor(() => expect(screen.getAllByText("yundrone-test01").length).toBeGreaterThan(0));
    fireEvent.click(screen.getByRole("tab", { name: "Wi-Fi 配网" }));
    fireEvent.change(screen.getByLabelText("SSID"), { target: { value: "LabWiFi" } });
    fireEvent.click(screen.getByRole("button", { name: "确认并下发配网" }));

    expect(await screen.findByRole("status", { name: "通知：配网成功" })).toBeInTheDocument();
  });

  it("shows disconnect errors in the top notification center", async () => {
    mocks.runCommand
      .mockResolvedValueOnce(statusResponse())
      .mockResolvedValueOnce(capabilitiesResponse());
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "连接设备" }));
    await waitFor(() => expect(screen.getAllByText("yundrone-test01").length).toBeGreaterThan(0));
    fireEvent.click(screen.getByRole("button", { name: "断开设备" }));

    expect(await screen.findByRole("status", { name: "通知：设备已断开" })).toBeInTheDocument();
  });
});

function unsupportedState(): BrowserSupportState {
  return {
    supported: false,
    secureContext: false,
    hasBluetoothApi: false,
    userAgent: "Mozilla/5.0 Chrome/120.0 Test",
    reason: "当前浏览器无法使用 Web Bluetooth，请使用桌面 Chrome/Edge 或 Android Chrome。",
  };
}
