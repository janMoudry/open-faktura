import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test("field errors are associated, loading is disabled, dialog restores focus", async ({
  page,
}) => {
  await page.goto("/sandbox.html#/design-system");
  await page.getByRole("button", { name: "Ověřit formulář" }).click();
  const field = page.getByLabel("Název zákazníka", { exact: true });
  await expect(field).toHaveAttribute("aria-invalid", "true");
  await expect(field).toHaveAccessibleDescription(/Vyplňte název zákazníka/);
  await field.fill("Studio Test");
  await page.getByRole("button", { name: "Ověřit formulář" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Studio Test" }),
  ).toContainText("Studio Test");
  await page.getByLabel("Ukázat stav načítání").check();
  await expect(page.getByRole("button", { name: "Ukládání…" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Ukládání…" })).toHaveAttribute(
    "aria-busy",
    "true",
  );
  await page.getByLabel("Ukázat stav načítání").uncheck();
  const trigger = page.getByRole("button", { name: "Otevřít ukázkový náhled" });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Ukázkový dialog" });
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "Zavřít dialog" }),
  ).toBeFocused();
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
  await page.keyboard.press("Shift+Tab");
  await expect(
    dialog.getByRole("button", { name: "Potvrdit ukázku" }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    dialog.getByRole("button", { name: "Zavřít dialog" }),
  ).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();
  await expect(trigger).toBeFocused();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

for (const route of ["design-system", "invoices", "invoices/new"]) {
  test(`accessible shared UI on ${route}`, async ({ page }) => {
    await page.goto(`/sandbox.html#/${route}`);
    await page.evaluate(() => document.fonts.ready);
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    expect(results.violations).toEqual([]);
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
  });
}

test("charts expose data and distinguish missing values from zero", async ({
  page,
}) => {
  await page.goto("/sandbox.html#/design-system");
  const chart = page.getByRole("figure", { name: "Vývoj počtu faktur" });
  await expect(chart.getByRole("application")).toBeVisible();
  const strokes = await chart
    .locator(".recharts-line-curve")
    .evaluateAll((paths) => paths.map((path) => getComputedStyle(path).stroke));
  expect(strokes).toHaveLength(2);
  expect(strokes.every((stroke) => stroke !== "none")).toBe(true);
  expect(new Set(strokes).size).toBe(2);
  await chart.getByRole("application").press("ArrowRight");
  await expect(chart.getByText("Vystavené: 32")).toBeVisible();
  await chart.getByText("Zobrazit data v tabulce").click();
  const table = chart.getByRole("table");
  await expect(table.getByRole("row", { name: "Říj 48 39" })).toBeVisible();
  await page.getByRole("button", { name: "Hraniční hodnoty" }).click();
  await expect(table.getByRole("row", { name: "Čvn 0 0" })).toBeVisible();
  await expect(
    table.getByRole("row", { name: "Čvc Bez dat Bez dat" }),
  ).toBeVisible();
  await expect(table.getByRole("row", { name: "Srp -8 -4" })).toBeVisible();
  await page.getByRole("button", { name: "Bez dat", exact: true }).click();
  await expect(
    chart.getByText("Pro toto období nejsou k dispozici žádná data."),
  ).toBeVisible();
  await expect(chart.getByRole("application")).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("tab indicator follows selection and reflow, respecting reduced motion", async ({
  page,
}) => {
  await page.goto("/sandbox.html#/design-system");
  const group = page.getByRole("group", { name: "Ukázkové stavy grafů" });
  await group.getByRole("button", { name: "Hraniční hodnoty" }).click();
  const indicator = group.locator(".ui-tab-indicator");
  await expect(indicator).toHaveCSS("transition-duration", "0.2s, 0.2s");
  const aligned = () =>
    group.evaluate((element) => {
      const selected = element
        .querySelector('[aria-pressed="true"]')!
        .getBoundingClientRect();
      const line = element
        .querySelector(".ui-tab-indicator")!
        .getBoundingClientRect();
      return (
        Math.abs(selected.left - line.left) < 1 &&
        Math.abs(selected.width - line.width) < 1 &&
        Math.abs(selected.bottom - line.bottom) < 1
      );
    });
  await expect.poll(aligned).toBe(true);
  await page.setViewportSize({ width: 360, height: 800 });
  await expect.poll(aligned).toBe(true);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await group.getByRole("button", { name: "Ukázková data" }).click();
  await expect(indicator).toHaveCSS("transition-duration", "0s");
  await expect.poll(aligned).toBe(true);
});
