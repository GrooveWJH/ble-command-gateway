import { readFileSync } from "node:fs";

export function readAppVersion() {
  return readFileSync(new URL("../VERSION", import.meta.url), "utf8").trim();
}
