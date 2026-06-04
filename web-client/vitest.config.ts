import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

import { readAppVersion } from "./buildVersion";

export default defineConfig({
  define: {
    __YUNDRONE_APP_VERSION__: JSON.stringify(readAppVersion()),
  },
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
  },
});
