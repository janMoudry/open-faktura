import { defineConfig, devices } from "@playwright/test";
const port = Number(process.env.E2E_WEB_PORT || 5173);
if (!Number.isInteger(port) || port < 1 || port > 65535)
  throw new Error("Invalid E2E_WEB_PORT");
const baseURL = `http://127.0.0.1:${port}`;
export default defineConfig({
  testDir: "./tests/e2e",
  use: { baseURL },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"] } },
    {
      name: "mobile",
      use: { ...devices["iPhone 13"], defaultBrowserType: "chromium" },
    },
  ],
  webServer: {
    command: `pnpm --filter @open-faktura/web dev --port ${port}`,
    url: baseURL,
    reuseExistingServer: !process.env.CI,
  },
});
