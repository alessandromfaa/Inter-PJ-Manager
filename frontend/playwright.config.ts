import { defineConfig } from "@playwright/test";

const origin = process.env.E2E_ORIGIN;
if (origin !== "http://127.0.0.1:18080")
  throw new Error("Playwright exige E2E_ORIGIN=http://127.0.0.1:18080.");
if (process.env.E2E_EMAIL !== "admin@e2e.local" || !process.env.E2E_PASSWORD)
  throw new Error("Playwright exige as credenciais da pilha E2E isolada.");

export default defineConfig({
  testDir: "./e2e",
  workers: 1,
  use: {
    baseURL: origin,
    launchOptions: { executablePath: process.env.CHROME_PATH },
  },
});
