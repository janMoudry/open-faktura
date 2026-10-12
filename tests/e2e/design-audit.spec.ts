import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/sandbox.html#/design-system");
  await page.evaluate(() => document.fonts.ready);
});
test("mixed checkbox, disabled filters and loading icon preserve states", async ({
  page,
}) => {
  const mixed = page.getByRole("checkbox", {
    name: "Částečně vybraná skupina",
  });
  await expect(mixed).toHaveAttribute("aria-checked", "mixed");
  await mixed.check();
  await expect(mixed).toBeChecked();
  await expect(mixed).not.toHaveAttribute("aria-checked", "mixed");
  const icon = page.getByRole("button", { name: "Načítající se ikonová akce" });
  await expect(icon).toBeDisabled();
  await expect(icon).toHaveAttribute("aria-busy", "true");
  await expect(icon.locator("svg")).toHaveCount(1);
  await expect(
    page.getByRole("button", { name: "Archiv (nedostupný)" }),
  ).toBeDisabled();
  await expect(
    page.getByRole("switch", { name: "Zakázaný zapnutý přepínač" }),
  ).toBeChecked();
});
test("chart loading and recoverable error hide stale data", async ({
  page,
}) => {
  const chart = page.getByRole("figure", { name: "Vývoj počtu faktur" });
  const tabs = page.getByRole("group", { name: "Ukázkové stavy grafů" });
  await tabs.getByRole("button", { name: "Načítání", exact: true }).click();
  await expect(chart).toHaveAttribute("aria-busy", "true");
  await expect(chart.getByRole("status")).toContainText("Načítání grafu");
  await expect(chart.getByRole("application")).toHaveCount(0);
  await tabs.getByRole("button", { name: "Chyba načítání" }).click();
  await expect(chart.getByRole("alert")).toContainText(
    "Graf se nepodařilo načíst",
  );
  await expect(chart.getByRole("table")).toHaveCount(0);
  await chart.getByRole("button", { name: "Zkusit načíst znovu" }).click();
  await expect(chart.getByRole("application")).toBeVisible();
  await expect(chart.getByRole("alert")).toHaveCount(0);
});
test("calendar fits 320px and readonly date cannot change", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 800 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  const readonly = page.getByRole("spinbutton", {
    name: "den, Datum pouze pro čtení",
    exact: true,
  });
  await readonly.press("ArrowUp");
  await expect(readonly).toHaveAttribute("aria-valuenow", "10");
  await expect(
    page.getByRole("button", {
      name: /Otevřít kalendář: Datum pouze pro čtení/,
    }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: /Otevřít kalendář: Ukázkové datum/ })
    .click();
  const calendar = page.getByRole("dialog", {
    name: "Kalendář: Ukázkové datum",
  });
  await expect(calendar).toBeVisible();
  const bounds = await calendar.boundingBox();
  expect(bounds!.x).toBeGreaterThanOrEqual(0);
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(320);
  expect(
    await calendar.evaluate((el) => el.scrollWidth <= el.clientWidth),
  ).toBe(true);
});
test("reduced motion removes control transitions and dialog entrance", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const toggle = page.getByRole("switch", {
    name: "Zapnout ukázkové oznámení",
  });
  await page.getByText("Zapnout ukázkové oznámení", { exact: true }).click();
  expect(
    await toggle.evaluate(
      (el) =>
        getComputedStyle(el.nextElementSibling!.firstElementChild!)
          .transitionDuration,
    ),
  ).toBe("0s");
  await page.getByRole("button", { name: "Otevřít ukázkový náhled" }).click();
  await expect(page.getByRole("dialog", { name: "Ukázkový dialog" })).toHaveCSS(
    "animation-name",
    "none",
  );
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: "Otevřít ukázkový náhled" }),
  ).toBeFocused();
});
