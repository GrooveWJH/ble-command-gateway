import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const shellStyles = readFileSync(join(process.cwd(), "src/styles/_shell.scss"), "utf8");
const headerStyles = readFileSync(join(process.cwd(), "src/styles/_header.scss"), "utf8");
const installStyles = readFileSync(join(process.cwd(), "src/styles/_install.scss"), "utf8");
const responsiveStyles = readFileSync(join(process.cwd(), "src/styles/_responsive.scss"), "utf8");

describe("shell styles", () => {
  it("centers text inside Carbon workspace tabs", () => {
    expect(shellStyles).toContain(".yd-tabs-frame .cds--tabs__nav-link");
    expect(shellStyles).toContain("display: inline-flex");
    expect(shellStyles).toContain("align-items: center");
    expect(shellStyles).toContain("justify-content: center");
    expect(shellStyles).toContain("text-align: center");
  });

  it("keeps the install script badge visibly glowing with reduced-motion fallback", () => {
    expect(installStyles).toContain(".yd-install-badge");
    expect(installStyles).toContain("yd-install-shimmer");
    expect(installStyles).toContain("yd-install-pulse");
    expect(installStyles).toContain("prefers-reduced-motion: reduce");
  });

  it("prevents install modal explanation tags from truncating their labels", () => {
    expect(installStyles).toContain(".yd-install-details .cds--tag");
    expect(installStyles).toContain("max-width: 100%");
    expect(installStyles).toContain(".yd-install-details .cds--tag__label");
    expect(installStyles).toContain("white-space: normal");
    expect(installStyles).toContain("text-overflow: clip");
  });

  it("keeps the language switch visible while constraining long English header text", () => {
    expect(headerStyles).toContain(".yd-language-action");
    expect(headerStyles).toContain("flex: 0 0 auto");
    expect(headerStyles).toContain("background: var(--cds-interactive)");
    expect(headerStyles).toContain(".yd-status-text");
    expect(headerStyles).toContain("max-width: clamp");
    expect(headerStyles).toContain("text-overflow: ellipsis");
    expect(responsiveStyles).toContain(".yd-header-status");
    expect(responsiveStyles).toContain("display: none");
    expect(responsiveStyles).toContain(".yd-header-actions");
    expect(responsiveStyles).toContain("display: flex");
  });
});
