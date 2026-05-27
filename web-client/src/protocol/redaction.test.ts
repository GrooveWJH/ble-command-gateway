import { describe, expect, it } from "vitest";

import { buildCommandRequest } from "./commands";
import { redactJson, redactSensitiveValue, traceBytes } from "./redaction";

describe("trace redaction", () => {
  it("redacts password-like fields in safe verbose mode", () => {
    const request = buildCommandRequest(
      "wifi.provision",
      { ssid: "Yundrone", pwd: "123123" },
      "fixed-id",
    );

    const safe = new TextDecoder().decode(traceBytes(request, true));

    expect(safe).toContain('"ssid":"Yundrone"');
    expect(safe).toContain('"pwd":"***"');
    expect(safe).not.toContain("123123");
    expect(redactJson({ nested: { password: "secret" } })).toEqual({
      nested: { password: "***" },
    });
  });

  it("preserves raw payloads only when unsafe mode is explicitly selected", () => {
    const request = buildCommandRequest(
      "wifi.provision",
      { ssid: "Yundrone", pwd: "123123" },
      "fixed-id",
    );

    expect(new TextDecoder().decode(traceBytes(request, false))).toContain('"pwd":"123123"');
    expect(redactSensitiveValue("token", "abc")).toBe("***");
  });
});
