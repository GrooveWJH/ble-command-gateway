import { describe, expect, it } from "vitest";

import { isStableYundroneName } from "./webBluetooth";

describe("isStableYundroneName", () => {
  it("accepts MAC-derived, diagnostic, and legacy YunDrone names", () => {
    for (const name of [
      "yundrone-12abcd",
      "yundrone-null",
      "edge [yundrone-12abcd]",
      "yundrone-ytcwln",
      "yundrone-lab1-k9x8",
      "yundrone-lab1k9x8",
      "edge [yundrone-lab1-k9x8]",
    ]) {
      expect(isStableYundroneName(name)).toBe(true);
    }
  });

  it("rejects invalid or unrelated names", () => {
    for (const name of [
      "Yundrone-lab1-k9x8",
      "yundrone-",
      "yundrone-lab_1",
      "yundrone-lab1--k9x8",
      "yundrone-07-44-5433",
      "gateway",
    ]) {
      expect(isStableYundroneName(name)).toBe(false);
    }
  });
});
