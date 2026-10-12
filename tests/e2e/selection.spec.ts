import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

async function accessible(
  page: import("@playwright/test").Page,
  comboboxOpen = false,
) {
  const axe = new AxeBuilder({ page }).withTags([
    "wcag2a",
    "wcag2aa",
    "wcag21aa",
  ]);
  // React Aria deliberately hides surroundings from AT while the input retains focus.
  // https://github.com/adobe/react-spectrum/wiki/Known-accessibility-false-positives
  // Only this open-combobox audit excludes the rule; Tab/cleanup and closed audits remain.
  if (comboboxOpen) axe.disableRules(["aria-hidden-focus"]);
  expect((await axe.analyze()).violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
}
test("custom select supports keyboard, disabled options and restores focus", async ({
  page,
}) => {
  await page.goto("/sandbox.html#/design-system");
  await page.evaluate(() => document.fonts.ready);
  const select = page.getByRole("button", {
    name: /Způsob platby/,
  });
  await select.press("Space");
  const list = page.getByRole("listbox");
  await expect(list).toBeVisible();
  await expect(page.getByRole("option", { name: "Kartou" })).toBeDisabled();
  await accessible(page);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(select).toContainText("Hotově");
  await expect(select).toBeFocused();
  await select.click();
  await page.keyboard.press("Escape");
  await expect(list).not.toBeVisible();
  await expect(select).toBeFocused();
  await expect(
    page.getByRole("button", { name: /Povinný výběr/ }),
  ).toHaveAccessibleDescription("Vyberte jednu možnost.");
});

test("autocomplete filters accents, selects and keeps custom text", async ({
  page,
}) => {
  await page.goto("/sandbox.html#/design-system");
  await page.evaluate(() => document.fonts.ready);
  const input = page.getByRole("combobox", {
    name: "Vyhledat kontakt",
    exact: true,
  });
  await input.scrollIntoViewIfNeeded();
  // Finish browser scrolling before opening the non-modal popup: React Aria
  // deliberately dismisses it on a parent scroll (including queued scroll events).
  await input.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  await input.focus();
  await expect(page.getByRole("listbox")).toBeVisible();
  await expect(page.getByRole("option").first()).toBeVisible();
  const initialCount = await page.getByRole("option").count();
  expect(initialCount).toBeGreaterThan(1);
  await input.fill("atelier");
  await expect(page.getByRole("option")).toHaveCount(1);
  await expect(
    page.getByRole("option", { name: /Ateliér Forma/ }),
  ).toBeVisible();
  await accessible(page, true);
  await input.press("ArrowDown");
  await input.press("Enter");
  await expect(input).toHaveValue("Ateliér Forma");
  await expect(input).toBeFocused();
  await input.fill("Nový zákazník");
  await expect(
    page.getByText("Žádné návrhy. Můžete zadat vlastní text."),
  ).toBeVisible();
  await input.press("Tab");
  await expect(input).toHaveValue("Nový zákazník");
  await expect(page.getByRole("listbox")).not.toBeVisible();
  await expect(page.getByLabel("Simulovat načítání návrhů")).toBeFocused();
  await accessible(page);
  await page.getByLabel("Simulovat načítání návrhů").check();
  await page
    .getByRole("button", {
      name: "Zobrazit návrhy Vyhledat kontakt",
      exact: true,
    })
    .click();
  await expect(
    page.getByText("Načítání návrhů…", { exact: true }),
  ).toBeVisible();
  await input.press("Escape");
});

test("custom calendar selects a date, edits segments and enforces limits", async ({
  page,
}) => {
  await page.goto("/sandbox.html#/design-system");
  await page.evaluate(() => document.fonts.ready);
  const trigger = page.getByRole("button", {
    name: /Otevřít kalendář: Ukázkové datum/,
  });
  await trigger.click();
  await expect(
    page.getByRole("dialog", { name: "Kalendář: Ukázkové datum" }),
  ).toBeVisible();
  await accessible(page);
  await page
    .getByRole("button", { name: "čtvrtek 15. října 2026", exact: true })
    .click();
  await expect(page.locator("output")).toHaveText("2026-10-15");
  await expect(trigger).toBeFocused();
  const day = page.getByRole("spinbutton", {
    name: "den, Ukázkové datum",
    exact: true,
  });
  await day.press("ArrowUp");
  await expect(page.locator("output")).toHaveText("2026-10-16");
  await page
    .getByRole("button", { name: /Otevřít kalendář: Omezený rozsah/ })
    .click();
  await expect(
    page.getByRole("button", { name: "pátek 9. října 2026", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "středa 21. října 2026", exact: true }),
  ).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).not.toBeVisible();
});

test("atomic switch and radio work from keyboard", async ({ page }) => {
  await page.goto("/sandbox.html#/design-system");
  await page.evaluate(() => document.fonts.ready);
  const toggle = page.getByRole("switch", {
    name: "Zapnout ukázkové oznámení",
  });
  await toggle.press("Space");
  await expect(toggle).toBeChecked();
  await expect(
    page.getByRole("switch", { name: "Zakázaný přepínač" }),
  ).toBeDisabled();
  await page
    .getByRole("radio", { name: "E-mailem", exact: true })
    .press("ArrowDown");
  await expect(
    page.getByRole("radio", { name: "Ke stažení", exact: true }),
  ).toBeChecked();
});
