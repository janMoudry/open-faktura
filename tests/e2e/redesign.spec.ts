import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdir } from "node:fs/promises";
import { mockRedesignWorkspace } from "./redesign-fixture";

for (const language of ["cs", "en"] as const) {
  test(`redesign: overview, register and editor retain accessible UI (${language})`, async ({
    page,
  }, info) => {
    test.setTimeout(90000);
    await mockRedesignWorkspace(page, { language });
    if (info.project.name === "desktop")
      await page.setViewportSize({ width: 1600, height: 1000 });
    for (const route of ["overview", "invoices", "invoices/draft"]) {
      await page.goto(`/#/companies/design/${route}`);
      await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
      if (route === "overview")
        await expect(
          page.getByRole("figure").getByRole("application"),
        ).toBeVisible();
      if (route === "invoices")
        await expect(page.getByRole("table")).toContainText("Studio Lípa");
      if (route === "invoices/draft") {
        await expect(page.locator(".invoice-editor-sheet")).toBeVisible();
        for (const row of await page.locator(".invoice-editor-line").all()) {
          expect(
            await row.evaluate((el) => el.scrollWidth <= el.clientWidth),
          ).toBe(true);
        }
      }
      await page.evaluate(() => document.fonts.ready);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      const results = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze();
      expect(results.violations).toEqual([]);
      if (process.env.DESIGN_CAPTURE === "1" && language === "cs") {
        await mkdir("docs/design/redesign-2026-10-17", { recursive: true });
        await page.screenshot({
          path: `docs/design/redesign-2026-10-17/${route.replaceAll("/", "-")}-${info.project.name}.png`,
          fullPage: true,
        });
      }
    }
    await page
      .getByRole("button", {
        name: language === "cs" ? "Uživatelské menu" : "Account menu",
        exact: true,
      })
      .click();
    const popover = page.locator(".account-popover");
    await expect(popover).toBeVisible();
    const box = await popover.boundingBox();
    expect(box!.y).toBeGreaterThanOrEqual(0);
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(
      (await page.viewportSize())!.width,
    );
    await page.keyboard.press("Escape");
    await expect(popover).not.toBeVisible();
  });
}

test("redesign: unavailable turnover stays unavailable and viewer has no create action", async ({
  page,
}) => {
  await mockRedesignWorkspace(page, { failReport: true, viewer: true });
  await page.goto("/#/companies/design/overview");
  await expect(
    page.getByText("Přehled prodeje a nákupu se nepodařilo načíst."),
  ).toBeVisible();
  await expect(page.getByRole("application")).toHaveCount(0);
  await expect(page.locator(".overview-metric").first()).not.toContainText(
    "0,00",
  );
  await expect(
    page.getByRole("link", { name: "Nová faktura", exact: true }),
  ).toHaveCount(0);
  await expect(page.locator(".overview-attention-link").first()).toContainText(
    "12",
  );
});

test("redesign: dark mode keeps brand and controls accessible", async ({
  page,
}, info) => {
  await mockRedesignWorkspace(page, { dark: true });
  await page.goto("/#/companies/design/invoices");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.getByRole("table")).toContainText("Studio Lípa");
  await expect(page.locator(".brand-logo-dark")).toBeVisible();
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
  if (process.env.DESIGN_CAPTURE === "1")
    await page.screenshot({
      path: `docs/design/redesign-2026-10-17/invoices-dark-${info.project.name}.png`,
      fullPage: true,
    });
});

test("redesign: header saves the form and a failed save retains edits", async ({
  page,
}) => {
  await mockRedesignWorkspace(page);
  await page.route("**/api/companies/design/invoices/draft", (route) =>
    route.request().method() === "PUT"
      ? route.fulfill({ status: 503, json: { code: "unavailable" } })
      : route.fallback(),
  );
  await page.goto("/#/companies/design/invoices/draft");
  const issue = page.getByRole("button", {
    name: "Vystavit fakturu",
    exact: true,
  });
  await expect(issue).toBeEnabled();
  const description = page.locator("[data-line-description]");
  await description.fill("Změněný popis položky");
  await expect(issue).toBeDisabled();
  const save = page.getByRole("button", { name: "Uložit návrh", exact: true });
  await expect(save).toBeEnabled();
  await save.click();
  await expect(page.getByRole("alert").first()).toBeVisible();
  await expect(description).toHaveValue("Změněný popis položky");
  await expect(issue).toBeDisabled();
});
