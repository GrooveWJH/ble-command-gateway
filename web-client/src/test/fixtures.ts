import type { CommandResponse } from "../types";

export function bluetoothConnection() {
  return {
    device: {
      name: "yundrone-test01",
      addEventListener() {},
      removeEventListener() {},
    },
    notifyCharacteristic: {
      addEventListener() {},
      removeEventListener() {},
    },
  };
}

export function statusResponse(): CommandResponse {
  return {
    id: "status-1",
    cmd: "system.status",
    phase: "result",
    seq: 1,
    final: true,
    ok: true,
    code: "OK",
    text: "status listed",
    data: {
      device_name: "yundrone-test01",
      hostname: "edge-gateway",
      system: "Ubuntu 20.04",
      user: "yundrone",
      network: "LabWiFi",
      ip: "192.0.2.2",
      interfaces: [
        { ifname: "wlan0", kind: "wifi", ipv4: "192.0.2.2" },
        { ifname: "eth0", kind: "ethernet", ipv4: "198.51.100.8" },
      ],
    },
    v: "YundroneBT-V2.1.0",
  };
}

export function provisionResponse(): CommandResponse {
  return {
    id: "provision-1",
    cmd: "wifi.provision",
    phase: "result",
    seq: 1,
    final: true,
    ok: true,
    code: "OK",
    text: "connected",
    data: { status: "connected", ssid: "LabWiFi" },
    v: "YundroneBT-V2.1.0",
  };
}

export function capabilitiesResponse(): CommandResponse {
  return {
    id: "capabilities-1",
    cmd: "system.capabilities",
    phase: "result",
    seq: 1,
    final: true,
    ok: true,
    code: "OK",
    text: "capabilities listed",
    data: {
      commands: [
        "link.heartbeat",
        "system.status",
        "system.capabilities",
        "wifi.scan",
        "wifi.provision",
      ],
      features: ["response_json_chunking", "wifi_profile_management"],
      payload_limit: 360,
      protocol_version: "YundroneBT-V2.1.0",
    },
    v: "YundroneBT-V2.1.0",
  };
}
