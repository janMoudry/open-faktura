import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdir } from "node:fs/promises";
import { mockRedesignWorkspace } from "./redesign-fixture";
import type { InvoiceDraft } from "@open-faktura/api-client";

for (const language of ["cs", "en"] as const) {
  test(`company default payment terms preselect new invoices (${language})`, async ({
    page,
  }, info) => {
    await page.clock.setFixedTime(new Date("2026-10-17T10:00:00Z"));
    const { company } = await mockRedesignWorkspace(page, { language });
    await page.route("**/api/companies/design", async (route) => {
      if (route.request().method() !== "PUT") return route.fallback();
      const input = route.request().postDataJSON();
      expect(input.revision).toBe(company.revision);
      company.settings = input.settings;
      company.revision++;
      return route.fulfill({ json: company });
    });
    const text = (cs: string, en: string) => (language === "cs" ? cs : en);
    for (const [days, expectedDay, expectedMonth] of [
      [30, "16", "11"],
      [0, "17", "10"],
      [19, "05", "11"],
    ] as const) {
      await page.goto("/#/companies/design");
      await page
        .getByRole("button", {
          name: text("Upravit nastavení", "Edit settings"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: text("Fakturační nastavení", "Billing preferences"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: new RegExp(text("Výchozí splatnost", "Default payment terms")),
        })
        .click();
      await page
        .getByRole("option", {
          name:
            days === 0
              ? text("Ihned", "Immediately")
              : days === 19
                ? text("Vlastní počet dní", "Custom number of days")
                : text("30 dní", "30 days"),
          exact: true,
        })
        .click();
      if (days === 19)
        await page
          .getByRole("spinbutton", {
            name: text("Splatnost (dny)", "Payment due in (days)"),
            exact: true,
          })
          .fill("19");
      if (days === 30 && language === "cs" && info.project.name === "desktop")
        await page.screenshot({
          path: "docs/design/redesign-2026-10-17/company-default-payment-terms.png",
          fullPage: true,
        });
      await page
        .getByRole("button", {
          name: text("Uložit fakturační údaje", "Save billing details"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Nastavení bylo uloženo.", "Settings saved."), {
          exact: true,
        }),
      ).toBeVisible();
      expect(company.settings.maturity_days).toBe(days);
      await page.reload();
      await page.goto("/#/companies/design/invoices/new");
      const terms = page.getByRole("button", {
        name: language === "cs" ? /Splatnost$/ : /Payment terms$/,
      });
      await expect(terms).toContainText(
        days === 0
          ? text("Ihned", "Immediately")
          : days === 19
            ? text("Vlastní datum", "Custom date")
            : text("30 dní", "30 days"),
      );
      const due = page.locator(".ui-selection").filter({
        has: page.getByText(text("Datum splatnosti", "Due date"), {
          exact: true,
        }),
      });
      await expect(due.getByRole("spinbutton").nth(0)).toHaveText(expectedDay);
      await expect(due.getByRole("spinbutton").nth(1)).toHaveText(
        expectedMonth,
      );
    }
  });
}

test("brand logo and brand breadcrumb return to the active company overview", async ({
  page,
}, info) => {
  await mockRedesignWorkspace(page);
  await page.goto("/#/companies/design/invoices");
  const logo = page.locator(".workspace-brand");
  await expect(logo).toHaveAttribute("href", "#/companies/design/overview");
  await logo.click();
  await expect(page).toHaveURL(/#\/companies\/design\/overview$/);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await page.goto("/#/companies/design");
  await expect(logo).toHaveAttribute("href", "#/companies/design/overview");
  if (info.project.name === "mobile") {
    await logo.click();
    await expect(page).toHaveURL(/#\/companies\/design\/overview$/);
    return;
  }
  const home = page
    .locator(".workspace-breadcrumb")
    .getByRole("link", { name: "Open Faktura", exact: true });
  await expect(home).toHaveAttribute("href", "#/companies/design/overview");
  await home.click();
  await expect(page).toHaveURL(/#\/companies\/design\/overview$/);
});

for (const language of ["cs", "en"] as const) {
  test(`invoice maturity days stay synchronized and survive saving (${language})`, async ({
    page,
  }, info) => {
    await mockRedesignWorkspace(page, { language });
    let stored: InvoiceDraft | undefined;
    await page.route(
      "**/api/companies/design/invoices/draft",
      async (route) => {
        if (route.request().method() === "PUT") {
          stored = {
            ...stored!,
            revision: stored!.revision + 1,
            data: route.request().postDataJSON().data,
          };
          return route.fulfill({ json: stored });
        }
        if (stored) return route.fulfill({ json: stored });
        return route.fallback();
      },
    );
    const response = page.waitForResponse((r) =>
      r.url().endsWith("/invoices/draft"),
    );
    await page.goto("/#/companies/design/invoices/draft");
    stored = await (await response).json();
    const terms = page.getByRole("button", {
      name: language === "cs" ? /Splatnost$/ : /Payment terms$/,
    });
    const custom = language === "cs" ? "Vlastní datum" : "Custom date";
    const preset = (days: number) =>
      `${days} ${language === "cs" ? "dní" : "days"}`;
    const choose = async (name: string) => {
      await terms.click();
      await page.getByRole("option", { name, exact: true }).click();
    };
    const issued = page.locator(".ui-selection").filter({
      has: page.getByText(
        language === "cs" ? "Datum vystavení" : "Issue date",
        { exact: true },
      ),
    });
    const due = page.locator(".ui-selection").filter({
      has: page.getByText(language === "cs" ? "Datum splatnosti" : "Due date", {
        exact: true,
      }),
    });
    await expect(terms).toContainText(preset(14));
    await terms.click();
    await expect(page.getByRole("option")).toHaveCount(7);
    const immediate = language === "cs" ? "Ihned" : "Immediately";
    await expect(
      page.getByRole("option", { name: immediate, exact: true }),
    ).toBeVisible();
    for (const days of [7, 14, 21, 30, 60])
      await expect(
        page.getByRole("option", { name: preset(days), exact: true }),
      ).toBeVisible();
    await page.getByRole("option", { name: preset(30), exact: true }).click();
    // October → November also crosses the Prague daylight-saving transition.
    await expect(due.getByRole("spinbutton").nth(0)).toHaveText("15");
    await expect(due.getByRole("spinbutton").nth(1)).toHaveText("11");
    await issued.getByRole("spinbutton").nth(0).press("ArrowUp");
    await expect(terms).toContainText(preset(30));
    await expect(due.getByRole("spinbutton").nth(0)).toHaveText("16");
    await due.getByRole("spinbutton").nth(0).fill("20");
    await expect(terms).toContainText(custom);
    // Manual entry stays custom even when it happens to match a preset.
    await due.getByRole("spinbutton").nth(0).fill("16");
    await expect(terms).toContainText(custom);
    await choose(preset(30));
    await expect(terms).toContainText(preset(30));
    await choose(custom);
    await expect(due.getByRole("spinbutton").nth(0)).toHaveText("16");
    await choose(preset(30));
    await page
      .getByRole("button", {
        name: language === "cs" ? "Uložit návrh" : "Save draft",
        exact: true,
      })
      .click();
    await expect(
      page.getByRole("status").filter({
        hasText:
          language === "cs"
            ? "Návrh faktury byl uložen."
            : "Invoice draft saved.",
      }),
    ).toBeVisible();
    expect(stored!.data.issued_on).toBe("2026-10-17");
    expect(stored!.data.due_on).toBe("2026-11-16");
    await page.reload();
    await expect(terms).toContainText(preset(30));
    if (language === "cs" && info.project.name === "desktop")
      await page.screenshot({
        path: "docs/design/redesign-2026-10-17/invoice-maturity-days.png",
        fullPage: true,
      });
    // Calendar arithmetic must handle leap days and crossing a year boundary.
    stored!.data.issued_on = "2028-02-28";
    stored!.data.due_on = "2028-03-13";
    await page.reload();
    await expect(terms).toContainText(preset(14));
    await choose(preset(7));
    await expect(due.getByRole("spinbutton").nth(0)).toHaveText("06");
    await expect(due.getByRole("spinbutton").nth(1)).toHaveText("03");
    stored!.data.issued_on = "2026-12-31";
    stored!.data.due_on = "2027-01-14";
    await page.reload();
    await expect(terms).toContainText(preset(14));
    await choose(preset(7));
    await expect(due.getByRole("spinbutton").nth(0)).toHaveText("07");
    await expect(due.getByRole("spinbutton").nth(1)).toHaveText("01");
    await expect(due.getByRole("spinbutton").nth(2)).toHaveText("2027");
    await choose(immediate);
    await expect(terms).toContainText(immediate);
    await expect(due.getByRole("spinbutton").nth(0)).toHaveText("31");
    await expect(due.getByRole("spinbutton").nth(1)).toHaveText("12");
    await expect(due.getByRole("spinbutton").nth(2)).toHaveText("2026");
    await issued.getByRole("spinbutton").nth(0).press("ArrowDown");
    await expect(due.getByRole("spinbutton").nth(0)).toHaveText("30");
    await expect(terms).toContainText(immediate);
    await page
      .getByRole("button", {
        name: language === "cs" ? "Uložit návrh" : "Save draft",
        exact: true,
      })
      .click();
    await expect(
      page.getByRole("status").filter({
        hasText:
          language === "cs"
            ? "Návrh faktury byl uložen."
            : "Invoice draft saved.",
      }),
    ).toBeVisible();
    expect(stored!.data.due_on).toBe(stored!.data.issued_on);
    await page.reload();
    await expect(terms).toContainText(immediate);
  });
}

test("appearance saves the selected theme and restores it after reload", async ({
  page,
}) => {
  await mockRedesignWorkspace(page, { language: "cs" });
  let preference = { revision: 0, theme: "light" };
  await page.route("**/api/auth/preferences", async (route) => {
    if (route.request().method() === "POST") {
      const input = route.request().postDataJSON();
      expect(input.revision).toBe(preference.revision);
      expect(route.request().headers()["x-faktura-request"]).toBe("1");
      preference = { revision: preference.revision + 1, theme: input.theme };
    }
    await route.fulfill({ json: preference });
  });
  await page.goto("/#/companies/design/invoices");
  await page
    .getByRole("button", { name: "Uživatelské menu", exact: true })
    .click();
  await page.getByRole("button", { name: /Vzhled/ }).click();
  await page.getByRole("option", { name: "Tmavý", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  expect(preference).toEqual({ revision: 1, theme: "dark" });
  await expect(page.locator(".account-popover").getByRole("alert")).toHaveCount(
    0,
  );
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page
    .getByRole("button", { name: "Uživatelské menu", exact: true })
    .click();
  await page.getByRole("button", { name: /Vzhled/ }).click();
  await page.getByRole("option", { name: "Světlý", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  expect(preference).toEqual({ revision: 2, theme: "light" });
});

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

test("avatar opens from the workspace and after scrolling at desktop breakpoints", async ({
  page,
}) => {
  await mockRedesignWorkspace(page);
  for (const width of [1024, 1280, 1600]) {
    await page.setViewportSize({ width, height: 720 });
    await page.goto("/#/workspace");
    const avatar = page.getByRole("button", {
      name: "Uživatelské menu",
      exact: true,
    });
    await expect(avatar).toBeVisible();
    for (const scroll of [0, 700]) {
      await page.evaluate((y) => window.scrollTo(0, y), scroll);
      const box = await page.locator(".workspace-account-avatar").boundingBox();
      expect(box).not.toBeNull();
      await page.mouse.click(box!.x + box!.width / 2, box!.y + box!.height / 2);
      await expect(avatar).toHaveAttribute("aria-expanded", "true");
      await expect(page.locator(".account-popover")).toBeVisible();
      await expect(page.locator(".account-popover")).toHaveCSS("opacity", "1");
      const menuBox = await page.locator(".account-popover").boundingBox();
      expect(menuBox!.height).toBeGreaterThan(200);
      expect(menuBox!.y).toBeGreaterThanOrEqual(0);
      expect(menuBox!.y + menuBox!.height).toBeLessThanOrEqual(720);
      if (
        width === 1280 &&
        scroll === 0 &&
        process.env.DESIGN_CAPTURE === "1"
      ) {
        await page.screenshot({
          path: "docs/design/redesign-2026-10-17/account-menu.png",
          animations: "disabled",
        });
      }
      await page.keyboard.press("Escape");
    }
  }
});
