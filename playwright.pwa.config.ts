import { defineConfig, devices } from "@playwright/test";
const port = Number(process.env.PWA_WEB_PORT || 4173);
if (!Number.isInteger(port) || port < 1 || port > 65535)
  throw new Error("Invalid PWA_WEB_PORT");
const baseURL = `http://127.0.0.1:${port}`;
// This configuration always targets a production preview, never Vite dev.
process.env.PWA_BASE_URL = baseURL;
export default defineConfig({
  testDir: "./tests/e2e",
  testMatch: "pwa.spec.ts",
  forbidOnly: !!process.env.CI,
  workers: 1,
  use: { baseURL, serviceWorkers: "allow", trace: "retain-on-failure" },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"] } },
    {
      name: "mobile",
      use: { ...devices["iPhone 13"], defaultBrowserType: "chromium" },
    },
  ],
  webServer: {
    command: `pnpm --filter @open-faktura/web exec vite preview --host 127.0.0.1 --port ${port} --strictPort`,
    url: baseURL,
    reuseExistingServer: !process.env.CI,
  },
});
