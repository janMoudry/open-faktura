import { test, expect } from "@playwright/test";

test("shows real API and PostgreSQL readiness without horizontal overflow", async ({
  page,
}) => {
  await page.goto("/#/system");
  await expect(page.getByText("● Připojeno")).toHaveCount(2);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("recovers after an unavailable API", async ({ page }) => {
  await page.route("**/api/*", (route) => route.abort());
  await page.goto("/#/system");
  await expect(page.getByRole("alert")).toBeVisible();
  await page.unroute("**/api/*");
  await page.getByRole("button", { name: "Ověřit spojení" }).click();
  await expect(page.getByText("● Připojeno")).toHaveCount(2);
});

test("does not report a ready database on HTTP 503", async ({ page }) => {
  await page.route("**/api/ready", (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: '{"status":"unavailable"}',
    }),
  );
  await page.goto("/#/system");
  await expect(page.getByText("● Připojeno")).toHaveCount(1);
  await expect(page.getByText("○ Nedostupné")).toHaveCount(1);
});
