import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import App from "./App";

describe("App shell", () => {
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

    expect(screen.getByLabelText("调试模式")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("过滤 trace")).toBeInTheDocument();
  });

  it("guides non-CLI users through the provisioning flow before connection", () => {
    render(<App />);

    expect(screen.getByRole("banner")).toHaveTextContent("YunDrone 配网工作台");
    expect(screen.getByText(/静态 HTTPS WebBluetooth 工具/)).toBeInTheDocument();
    const steps = screen.getByLabelText("配网步骤");
    expect(within(steps).getByText("环境检查")).toBeInTheDocument();
    expect(within(steps).getByText("连接设备")).toBeInTheDocument();
    expect(within(steps).getByText("扫描 Wi-Fi")).toBeInTheDocument();
    expect(within(steps).getByText("选择网络")).toBeInTheDocument();
    expect(within(steps).getByText("输入密码")).toBeInTheDocument();
    expect(screen.getByText("先连接设备")).toBeInTheDocument();
    expect(screen.getByText(/在浏览器蓝牙选择器中选择名称以 yundrone-/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认并下发配网" })).toBeDisabled();
    expect(screen.getByText(/设备保持上电并靠近电脑或手机/)).toBeInTheDocument();
    expect(screen.getByText(/尚未扫描 Wi-Fi。也可以直接在上方 SSID 输入框填写隐藏网络/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText("搜索 SSID")).toBeDisabled();
    expect(screen.getByText(/密码：未填写，适用于开放网络/)).toBeInTheDocument();
    expect(screen.getByText(/无需安装 CLI/)).toBeInTheDocument();
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

    fireEvent.change(screen.getByPlaceholderText("过滤 trace"), {
      target: { value: "assembled" },
    });
    fireEvent.click(screen.getByRole("button", { name: "展开" }));

    expect(screen.queryByText(/request system.status/)).not.toBeInTheDocument();
    expect(screen.getByText(/wifi.scan result/)).toBeInTheDocument();
  });
});
