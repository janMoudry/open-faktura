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

test("child routes retain profile data, navigation controls and expanded groups", async ({
  page,
}) => {
  const settings = {
    name: "Navigation fixture",
    entity_type: "individual",
    registration_number: "",
    vat_number: "",
    street: "",
    city: "",
    postal_code: "",
    country: "CZ",
    email: "",
    phone: "",
    tax_mode: "non_payer",
    tax_effective_from: "",
    currency: "CZK",
    timezone: "Europe/Prague",
    payment_method: "cash",
    iban: "",
    number_prefix: "FV",
    maturity_days: 14,
    number_year: 2026,
    next_number: 1,
  };
  const profile = {
    id: "1",
    revision: 1,
    step: 0,
    completed: true,
    role: "owner",
    settings,
    missing_fields: [],
    number_preview: "FV2026000001",
    can_issue: true,
    tax_history: [],
  };
  let profileReads = 0;
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/setup")
      return route.fulfill({ json: { setup_required: false } });
    if (path === "/api/auth/session")
      return route.fulfill({
        json: {
          id: "routing-user",
          name: "Test",
          email: "routing@example.test",
          installation_admin: false,
        },
      });
    if (path === "/api/companies" || path === "/api/companies/1") {
      profileReads++;
      return route.fulfill({
        json: path === "/api/companies" ? [profile] : profile,
      });
    }
    return route.fulfill({ status: 503, json: { code: "unavailable" } });
  });
  await page.goto("/#/companies/1/invoices");
  const sidebar = page.locator(".workspace-sidebar");
  const create = sidebar.locator('a[href$="/invoices/new"]');
  await expect(create).toBeVisible();
  const node = await create.elementHandle();
  const initialProfileReads = profileReads;
  const mobile = sidebar.locator(":scope > details");
  if (await mobile.isVisible()) await mobile.locator("summary").first().click();
  const sales = sidebar
    .locator(".workspace-nav-group:visible")
    .filter({ has: page.locator("summary", { hasText: "Prodej" }) });
  const directory = sidebar
    .locator(".workspace-nav-group:visible")
    .filter({ has: page.locator("summary", { hasText: "Adresář" }) });
  await directory.locator("summary").click();
  await sidebar
    .getByRole("link", { name: "Odběratelé", exact: true })
    .filter({ visible: true })
    .click();
  await expect(page).toHaveURL(/\/contacts$/);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(sales).toHaveAttribute("open", "");
  expect(
    await create.evaluate((current, previous) => current === previous, node),
  ).toBe(true);
  expect(profileReads).toBe(initialProfileReads);
  await sidebar
    .getByRole("link", { name: "Faktury", exact: true })
    .filter({ visible: true })
    .click();
  await expect(page).toHaveURL(/\/invoices$/);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(directory).toHaveAttribute("open", "");
  expect(
    await create.evaluate((current, previous) => current === previous, node),
  ).toBe(true);
  expect(profileReads).toBe(initialProfileReads);
  await page.goBack();
  await expect(page).toHaveURL(/\/contacts$/);
  expect(profileReads).toBe(initialProfileReads);
});
