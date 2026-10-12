import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

async function mockSetup(
  page: import("@playwright/test").Page,
  required = true,
) {
  await page.route("**/api/auth/setup", (r) =>
    r.fulfill({ json: { setup_required: required } }),
  );
  await page.route("**/api/auth/session", (r) =>
    r.fulfill({ status: 401, json: { code: "unauthorized" } }),
  );
}

test("language switch preserves form and validation, persists through reload", async ({
  page,
}) => {
  await mockSetup(page);
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("lang", "cs");
  await page.getByLabel("E-mail", { exact: true }).fill("test@example.invalid");
  await page.getByLabel("Heslo", { exact: true }).fill("test-password-long");
  await page
    .getByLabel("Zopakujte heslo", { exact: true })
    .fill("different-password");
  await page
    .getByLabel("Instalační klíč", { exact: true })
    .fill("synthetic-test-key-not-a-real-secret-123");
  await page
    .getByRole("button", { name: "Založit správce", exact: true })
    .click();
  await expect(page.getByRole("alert")).toHaveText("Hesla se neshodují.");
  await page.getByRole("button", { name: "English", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Create an administrator",
  );
  await expect(page.getByRole("alert")).toHaveText("Passwords do not match.");
  await expect(page.getByLabel("Email", { exact: true })).toHaveValue(
    "test@example.invalid",
  );
  await expect(page.getByLabel("Password", { exact: true })).toHaveValue(
    "test-password-long",
  );
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await page.getByLabel("Confirm password", { exact: true }).press("Tab");
  await expect(page.getByRole("tooltip")).toContainText("Where can I find it?");
  await page.keyboard.press("Escape");
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
  await page.reload();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Create an administrator",
  );
  await page.getByRole("button", { name: "Čeština", exact: true }).click();
  await page.reload();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Založte správce instalace",
  );
});

for (const language of ["cs", "en"] as const) {
  test(`setup and sign-in complete in ${language} with mocked API`, async ({
    page,
  }) => {
    await mockSetup(page);
    await page.route("**/api/companies", (r) => r.fulfill({ json: [] }));
    await page.addInitScript(
      (lang) => localStorage.setItem("open-faktura.language", lang),
      language,
    );
    let signedIn = false;
    await page.route("**/api/auth/session", (r) =>
      signedIn
        ? r.fulfill({
            json: {
              id: "test-user",
              email: "test@example.invalid",
              installation_admin: true,
              name: "",
            },
          })
        : r.fulfill({ status: 401, json: { code: "unauthorized" } }),
    );
    await page.route("**/api/auth/setup", (r) =>
      r.request().method() === "POST"
        ? r.fulfill({ status: 201, json: {} })
        : r.fulfill({ json: { setup_required: !signedIn } }),
    );
    await page.route("**/api/auth/login", (r) => {
      signedIn = true;
      return r.fulfill({
        json: {
          id: "test-user",
          email: "test@example.invalid",
          installation_admin: true,
          name: "",
        },
      });
    });
    await page.goto("/");
    const english = language === "en";
    await page
      .getByLabel(english ? "Email" : "E-mail", { exact: true })
      .fill("test@example.invalid");
    await page
      .getByLabel(english ? "Password" : "Heslo", { exact: true })
      .fill("test-password-long");
    await page
      .getByLabel(english ? "Confirm password" : "Zopakujte heslo", {
        exact: true,
      })
      .fill("test-password-long");
    await page
      .getByLabel(english ? "Installation key" : "Instalační klíč", {
        exact: true,
      })
      .fill("synthetic-test-key-not-a-real-secret-123");
    await page
      .getByRole("button", {
        name: english ? "Create administrator" : "Založit správce",
        exact: true,
      })
      .click();
    await expect(page.getByRole("status")).toContainText(
      english ? "Administrator created" : "Správce byl založen",
    );
    await page
      .getByLabel(english ? "Password" : "Heslo", { exact: true })
      .fill("test-password-long");
    await page
      .getByRole("button", {
        name: english ? "Sign in" : "Přihlásit se",
        exact: true,
      })
      .click();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      english ? "Billing profiles" : "Fakturační profily",
    );
  });
}

test("unknown errors and unsupported stored language use safe fallbacks", async ({
  page,
}) => {
  await mockSetup(page, false);
  await page.addInitScript(() =>
    localStorage.setItem("open-faktura.language", "de"),
  );
  await page.route("**/api/auth/login", (r) =>
    r.fulfill({ status: 500, json: { code: "unknown_future_code" } }),
  );
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("lang", "cs");
  await page.getByRole("button", { name: "English", exact: true }).click();
  await page.getByLabel("Email", { exact: true }).fill("test@example.invalid");
  await page.getByLabel("Password", { exact: true }).fill("test-password-long");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "The server is unavailable",
  );
  await page.getByRole("button", { name: "Čeština", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Server není dostupný");
});

test("language still switches when storage is unavailable", async ({
  page,
}) => {
  await mockSetup(page);
  await page.addInitScript(() => {
    Storage.prototype.getItem = () => {
      throw new Error("Storage disabled");
    };
    Storage.prototype.setItem = () => {
      throw new Error("Storage disabled");
    };
  });
  await page.goto("/");
  await page.getByRole("button", { name: "English", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Create an administrator",
  );
});

test("shared calendar uses English locale", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("open-faktura.language", "en"),
  );
  await page.goto("/sandbox.html#/design-system");
  await page
    .getByRole("button", { name: /^Open calendar:/ })
    .first()
    .click();
  await expect(page.getByRole("button", { name: "Next month" })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Previous month" }),
  ).toBeVisible();
});

test("locale formatting keeps exact money and calendar dates; Czech plurals work", async ({
  page,
}) => {
  await mockSetup(page);
  await page.goto("/");
  const result = await page.evaluate(async (root) => {
    const format = await import(`${root}/format.ts`);
    const { translateUi } = await import(`${root}/index.tsx`);
    return {
      cs: format.formatNumber(1234.5, "cs"),
      en: format.formatNumber(1234.5, "en"),
      money: format.formatMoney("9007199254740993.12", "GBP", "en"),
      date: format.formatCalendarDate("2026-03-29", "en"),
      plurals: [1, 2, 5].map((count) => translateUi("cs", "series", { count })),
    };
  }, `/@fs${process.cwd()}/packages/ui/src/locales`);
  expect(result.cs.replaceAll("\u00a0", " ")).toBe("1 234,5");
  expect(result.en).toBe("1,234.5");
  expect(result.money).toBe("£9,007,199,254,740,993.12");
  expect(result.date).toBe("29/03/2026");
  expect(result.plurals).toEqual([
    "1 datová řada",
    "2 datové řady",
    "5 datových řad",
  ]);
});
