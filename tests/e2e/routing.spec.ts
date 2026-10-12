import { test, expect } from "@playwright/test";

test("workspace shell survives lazy loading, navigation, back and failed page data", async ({
  page,
}) => {
  let authRequests = 0;
  await page.route("**/api/auth/setup", (r) => {
    authRequests++;
    return r.fulfill({ json: { setup_required: false } });
  });
  await page.route("**/api/auth/session", (r) => {
    authRequests++;
    return r.fulfill({
      json: {
        id: "routing-user",
        name: "Test",
        email: "routing@example.test",
        installation_admin: false,
      },
    });
  });
  let fail = false;
  let hold = false;
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/companies", async (r) => {
    if (hold) await pending;
    await r.fulfill(
      fail ? { status: 503, json: { code: "unavailable" } } : { json: [] },
    );
  });
  let releaseModule!: () => void;
  const moduleReady = new Promise<void>((resolve) => {
    releaseModule = resolve;
  });
  await page.route("**/src/pages/CompanyPage.tsx*", async (r) => {
    await moduleReady;
    await r.continue();
  });
  try {
    await page.goto("/#/workspace", { waitUntil: "domcontentloaded" });
    const sidebar = page.locator(".workspace-sidebar");
    await expect(sidebar).toBeVisible();
    await expect(page.getByRole("status")).toContainText("Načít");
    const sidebarNode = await sidebar.elementHandle();
    const headerNode = await page.locator("header").elementHandle();
    releaseModule();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Fakturační profily",
    );
    // Open the visible sidebar on mobile and retain its state across routes.
    const mobile = sidebar.locator(":scope > details");
    if (await mobile.isVisible())
      await mobile.locator("summary").first().click();
    const sales = sidebar
      .locator(".workspace-nav-group:visible")
      .filter({ has: page.locator("summary", { hasText: "Prodej" }) })
      .last();
    await sales.locator("summary").first().click();
    const initialAuthRequests = authRequests;
    hold = true;
    await page.getByRole("button", { name: /Fakturuji jako/ }).click();
    await page.getByRole("link", { name: "Spravovat profily" }).click();
    await expect(page).toHaveURL(/#\/companies$/);
    await expect(page.getByRole("status")).toContainText("Načít");
    expect(
      await sidebar.evaluate((node, old) => node === old, sidebarNode),
    ).toBe(true);
    expect(
      await page
        .locator("header")
        .evaluate((node, old) => node === old, headerNode),
    ).toBe(true);
    await expect(sales).toHaveAttribute("open", "");
    release();
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await page.goBack();
    await expect(page).toHaveURL(/#\/workspace$/);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    expect(
      await sidebar.evaluate((node, old) => node === old, sidebarNode),
    ).toBe(true);
    expect(authRequests).toBe(initialAuthRequests);
    fail = true;
    await page.getByRole("button", { name: /Fakturuji jako/ }).click();
    await page.getByRole("link", { name: "Spravovat profily" }).click();
    await expect(page.getByRole("alert")).toBeVisible();
    expect(
      await sidebar.evaluate((node, old) => node === old, sidebarNode),
    ).toBe(true);
    await expect(sales).toHaveAttribute("open", "");
  } finally {
    releaseModule();
    release();
    await page.unrouteAll({ behavior: "wait" });
  }
});
