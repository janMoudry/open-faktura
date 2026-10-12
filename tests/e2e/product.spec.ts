import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test("new installation shows real setup without demo content", async ({
  page,
}) => {
  await page.route("**/api/auth/setup", (r) =>
    r.fulfill({ json: { setup_required: true } }),
  );
  await page.route("**/api/auth/session", (r) =>
    r.fulfill({ status: 401, json: { code: "unauthorized" } }),
  );
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Založte správce instalace",
  );
  await expect(page.getByText("Studio Sever")).toHaveCount(0);
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("direct workspace routes require a session and never expose sandbox", async ({
  page,
}) => {
  await page.route("**/api/auth/setup", (r) =>
    r.fulfill({ json: { setup_required: false } }),
  );
  await page.route("**/api/auth/session", (r) =>
    r.fulfill({ status: 401, json: { code: "unauthorized" } }),
  );
  for (const path of ["login", "setup", "companies", "workspace", "invoices"]) {
    await page.goto(`/#/${path}`);
    await page.reload();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Přihlášení",
    );
    await expect(
      page.getByRole("button", { name: "Nová faktura" }),
    ).toHaveCount(0);
  }
  await page.goto("/#/design-system");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Tuto stránku jsme nenašli",
  );
});

test("database failure never starts bootstrap", async ({ page }) => {
  await page.route("**/api/auth/**", (r) =>
    r.fulfill({ status: 503, json: { code: "unavailable" } }),
  );
  await page.goto("/");
  await expect(page.getByRole("alert")).toContainText(
    "Nepodařilo se ověřit stav instalace",
  );
  await expect(
    page.getByRole("button", { name: "Založit správce" }),
  ).toHaveCount(0);
  await page.unroute("**/api/auth/**");
  await page.getByRole("button", { name: "Zkusit znovu" }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    /Založte správce instalace|Přihlášení/,
  );
});

test("setup key help is an icon with hover and keyboard tooltip", async ({
  page,
}) => {
  await page.route("**/api/auth/setup", (r) =>
    r.fulfill({ json: { setup_required: true } }),
  );
  await page.route("**/api/auth/session", (r) =>
    r.fulfill({ status: 401, json: { code: "unauthorized" } }),
  );
  await page.goto("/");
  const icon = page.getByRole("img", { name: "Nápověda: Instalační klíč" });
  await expect(
    page.getByRole("button", { name: "Nápověda: Instalační klíč" }),
  ).toHaveCount(0);
  await page.mouse.move(0, 0);
  await icon.scrollIntoViewIfNeeded();
  await icon.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  await icon.hover();
  const tooltip = page.getByRole("tooltip");
  await expect(tooltip).toContainText("Kde ho najdu?");
  await expect(tooltip).toContainText(".secrets/bootstrap");
  await page.mouse.move(0, 0);
  await expect(tooltip).not.toBeVisible();
  await page.getByLabel("Zopakujte heslo").focus();
  await page.keyboard.press("Tab");
  await expect(icon).toBeFocused();
  await expect(tooltip).toBeVisible();
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
  expect(
    await tooltip.evaluate((element) => {
      const r = element.getBoundingClientRect();
      return r.left >= 0 && r.right <= innerWidth;
    }),
  ).toBe(true);
  await page.keyboard.press("Escape");
  await expect(tooltip).not.toBeVisible();
  await expect(icon).toBeFocused();
});
