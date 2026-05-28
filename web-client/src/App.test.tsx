import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { bluetoothConnection, capabilitiesResponse, provisionResponse, statusResponse } from "./test/fixtures";
import App from "./App";
import type { BrowserSupportState } from "./types";

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

function enableBluetoothSupport() {
  mocks.support = { supported: true, secureContext: true, hasBluetoothApi: true, userAgent: TEST_USER_AGENT };
}

function disableBluetoothSupport() { mocks.support = unsupportedState(); }

function unsupportedState(): BrowserSupportState {
  return {
    supported: false,
    secureContext: false,
    hasBluetoothApi: false,
    userAgent: "Mozilla/5.0 Chrome/120.0 Test",
    reason: "当前浏览器无法使用 Web Bluetooth，请使用桌面 Chrome/Edge 或 Android Chrome。",
  };
}

describe("App shell", () => {
  beforeEach(() => {
    vi.useRealTimers();
    disableBluetoothSupport();
    mocks.requestDevice.mockReset();
    mocks.requestDevice.mockResolvedValue(bluetoothConnection());
    mocks.runCommand.mockReset();
    mocks.disconnect.mockReset();
    vi.spyOn(window, "confirm").mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
    window.localStorage.clear();
  });

  it("shows a clear unsupported browser notice when Web Bluetooth is unavailable", () => {
    render(<App />);

    expect(screen.getByText("当前浏览器无法使用 Web Bluetooth")).toBeInTheDocument();
    expect(screen.getByText(/Google Chrome 或 Android Chrome/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "连接设备" })).not.toBeDisabled();
  });

  it("keeps the connect trigger actionable when Web Bluetooth is unavailable", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "连接设备" }));

    expect(screen.getByText("当前浏览器不可用")).toBeInTheDocument();
  });

  it("keeps debug controls visible in the unsupported state", () => {
    render(<App />);

    expect(screen.getByText(/Trace 记录/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "展开调试抽屉" })).toBeInTheDocument();
  });

  it("puts basic information first and keeps provisioning steps inside the Wi-Fi page", () => {
    render(<App />);

    expect(screen.getByRole("banner")).toHaveTextContent("YunDrone 配网工作台");
    expect(screen.getByText(/连接已部署被控端/)).toBeInTheDocument();
    expect(tabLabels()).toEqual(["基本信息", "Wi-Fi 配网", "Wi-Fi 管理", "高级命令"]);
    expect(screen.getAllByRole("tab")[0]).toHaveAttribute("aria-selected", "true");
    fireEvent.click(screen.getByRole("tab", { name: "Wi-Fi 配网" }));
    expect(stepLabels()).toEqual(["环境检查", "连接设备", "扫描 Wi-Fi", "选择网络", "输入密码"]);
    expect(screen.getByText("先连接设备")).toBeInTheDocument();
    expect(screen.getByText(/在浏览器蓝牙选择器中选择名称以 yundrone-/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认并下发配网" })).toBeDisabled();
    expect(screen.getByText(/设备保持上电并靠近电脑或手机/)).toBeInTheDocument();
    expect(screen.getByText(/尚未扫描 Wi-Fi。也可以直接在上方 SSID 输入框填写隐藏网络/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText("搜索 SSID")).toBeDisabled();
    expect(screen.getByText(/密码：未填写，适用于开放网络/)).toBeInTheDocument();
    expect(screen.getByText(/无需安装 CLI/)).toBeInTheDocument();
  });

  it("uses a fixed viewport workspace without a global journey rail", () => {
    const { container } = render(<App />);

    expect(container.querySelector(".yd-operator-grid")).not.toBeInTheDocument();
    expect(container.querySelector(".yd-journey-rail")).not.toBeInTheDocument();
    const workspace = container.querySelector(".yd-workspace-shell");
    expect(workspace).toBeInTheDocument();
    expect(workspace?.querySelector(":scope > .cds--tab-content")).not.toBeInTheDocument();
  });

  it("filters debug trace rows", () => {
    render(
      <App
        initialTrace={[
          {
            id: "1",
            at: "10:00:00",
            level: "info",
            label: "TX:raw",
            summary: "request system.status",
          },
          {
            id: "2",
            at: "10:00:01",
            level: "info",
            label: "RX:assembled",
            summary: "wifi.scan result",
          },
        ]}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "展开调试抽屉" }));
    fireEvent.change(screen.getByPlaceholderText("过滤 trace"), {
      target: { value: "assembled" },
    });

    expect(screen.queryByText(/request system.status/)).not.toBeInTheDocument();
    expect(screen.getByText(/wifi.scan result/)).toBeInTheDocument();
  });

  it("shows only one debug drawer collapse control when expanded", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "展开调试抽屉" }));

    expect(screen.getAllByRole("button", { name: "收起调试抽屉" })).toHaveLength(1);
  });

  it("opens basic information and loads status plus capabilities after connection", async () => {
    enableBluetoothSupport();
    mocks.runCommand
      .mockResolvedValueOnce(statusResponse())
      .mockResolvedValueOnce(capabilitiesResponse());
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "连接设备" }));
    await waitFor(() => expect(screen.getAllByText("yundrone-test01").length).toBeGreaterThan(0));

    await waitFor(() => {
      expect(mocks.runCommand).toHaveBeenNthCalledWith(1, "system.status", {});
      expect(mocks.runCommand).toHaveBeenNthCalledWith(2, "system.capabilities", {});
    });
    expect(screen.getByRole("tab", { name: "基本信息" })).toHaveAttribute("aria-selected", "true");
    expect(await screen.findByText("首选 IP")).toBeInTheDocument();
    expect(screen.getAllByText("192.0.2.2").length).toBeGreaterThan(0);
    expect(screen.getByText("wlan0")).toBeInTheDocument();
    expect(screen.getAllByText("YundroneBT-V2.1.0").length).toBeGreaterThan(0);
    expect(screen.queryByText("wifi.scan")).not.toBeInTheDocument();
    fireEvent.click(screen.getByText("高级协议详情"));
    expect(screen.getByText("wifi.scan")).toBeInTheDocument();
    expect(screen.queryByText(/\"hostname\"/)).not.toBeInTheDocument();
  });

  it("shows a centered overlay while the browser is connecting to BLE", async () => {
    enableBluetoothSupport();
    mocks.requestDevice.mockReturnValue(new Promise(() => undefined));
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "连接设备" }));

    expect(await screen.findByRole("status", { name: "BLE 操作进行中" })).toHaveTextContent("正在连接被控端…");
    expect(screen.getByTestId("ble-command-overlay")).toHaveClass("yd-ble-overlay");
  });

  it("shows command-specific overlay feedback while a BLE command is running", async () => {
    enableBluetoothSupport();
    mocks.runCommand
      .mockResolvedValueOnce(statusResponse())
      .mockResolvedValueOnce(capabilitiesResponse())
      .mockReturnValueOnce(new Promise(() => undefined));
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "连接设备" }));
    await waitFor(() => expect(screen.getAllByText("yundrone-test01").length).toBeGreaterThan(0));
    await waitFor(() => expect(mocks.runCommand).toHaveBeenNthCalledWith(2, "system.capabilities", {}));
    fireEvent.click(screen.getByRole("tab", { name: "Wi-Fi 配网" }));
    fireEvent.click(screen.getByRole("button", { name: "扫描 Wi-Fi" }));

    const overlay = await screen.findByRole("status", { name: "BLE 操作进行中" });
    expect(overlay).toHaveTextContent("正在扫描 Wi-Fi…");
    expect(screen.getByRole("button", { name: "扫描 Wi-Fi" })).toBeDisabled();
    expect(screen.getAllByRole("status", { name: "BLE 操作进行中" })).toHaveLength(1);
    expect(screen.queryByText("命令执行中")).not.toBeInTheDocument();
    expect(screen.getAllByText("正在扫描 Wi-Fi…")).toHaveLength(1);
  });

  it("refreshes system status and surfaces the device IP after provisioning", async () => {
    enableBluetoothSupport();
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

    await waitFor(() => {
      expect(mocks.runCommand).toHaveBeenNthCalledWith(3, "wifi.provision", {
        ssid: "LabWiFi",
        pwd: "",
      });
      expect(mocks.runCommand).toHaveBeenNthCalledWith(4, "system.status", {});
    });
    expect(screen.getByRole("tab", { name: "基本信息" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getAllByText("192.0.2.2").length).toBeGreaterThan(0);
  });
});

function tabLabels() { return screen.getAllByRole("tab").map((tab) => tab.textContent); }

function stepLabels() {
  const steps = screen.getByLabelText("配网步骤");
  return ["环境检查", "连接设备", "扫描 Wi-Fi", "选择网络", "输入密码"]
    .filter((label) => within(steps).queryByText(label));
}
