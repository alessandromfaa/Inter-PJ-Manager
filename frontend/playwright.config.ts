import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./e2e",
  workers: 1,
  use: {
    baseURL: process.env.E2E_ORIGIN || "http://127.0.0.1:8080",
    launchOptions: { executablePath: process.env.CHROME_PATH },
  },
});
