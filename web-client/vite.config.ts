import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

import { readAppVersion } from "./buildVersion";

export default defineConfig({
  base: "/ble/",
  define: {
    __YUNDRONE_APP_VERSION__: JSON.stringify(readAppVersion()),
  },
  plugins: [react()],
});
