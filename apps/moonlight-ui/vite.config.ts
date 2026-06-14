import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  base: process.env.VITE_MOONLIGHT_BASE_PATH ?? "/",
  plugins: [react()],
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8080"
    }
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: "./src/test/setup.ts"
  }
});
