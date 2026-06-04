import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const indexHtml = readFileSync(join(process.cwd(), "index.html"), "utf8");

describe("static shell", () => {
  it("defaults the document shell to English before React hydrates", () => {
    expect(indexHtml).toContain('<html lang="en">');
    expect(indexHtml).toContain("<title>YunDrone BLE Wi-Fi Tool</title>");
  });

  it("uses the Bluetooth favicon served from the /ble/ base path", () => {
    expect(indexHtml).toContain('rel="icon"');
    expect(indexHtml).toContain('href="/ble/favicon.svg"');
  });
});
