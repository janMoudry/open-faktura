import { test, expect } from "@playwright/test";
for (const language of ["cs", "en"] as const) {
  test(`help guides, navigation and connection checks ${language}`, async ({
    page,
  }, info) => {
    await page.addInitScript(
      (lang) => localStorage.setItem("open-faktura.language", lang),
      language,
    );
    await page.route("**/api/auth/setup", (r) =>
      r.fulfill({ json: { setup_required: false } }),
    );
    await page.route("**/api/auth/session", (r) =>
      r.fulfill({
        json: {
          id: "help-test",
          name: "Help Test",
          email: "help@example.test",
          installation_admin: false,
        },
      }),
    );
    await page.route("**/api/companies", (r) => r.fulfill({ json: [] }));
    await page.route("**/api/preferences", (r) =>
      r.fulfill({ json: { appearance: "light" } }),
    );
    let connected = true;
    await page.route("**/health", (r) => r.fulfill({ json: { status: "ok" } }));
    await page.route("**/ready", (r) =>
      connected
        ? r.fulfill({ json: { status: "ready" } })
        : r.fulfill({ status: 503, json: { status: "unavailable" } }),
    );
    await page.goto("/#/help");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      language === "cs" ? "Nápověda" : "Help",
    );
    await expect(
      page.getByRole("link", {
        name: language === "cs" ? "Otevřít v aplikaci" : "Open in application",
      }),
    ).toHaveCount(6);
    const check = page.getByRole("button", {
      name:
        language === "cs" ? "Znovu ověřit připojení" : "Check connection again",
    });
    await expect(check).toBeEnabled();
    await expect(
      page.getByText(language === "cs" ? "● Připojeno" : "● Connected", {
        exact: true,
      }),
    ).toHaveCount(2);
    await page.screenshot({
      path: `docs/design/help-${language}-${info.project.name}.png`,
      fullPage: true,
    });
    connected = false;
    await check.click();
    await expect(check).toBeEnabled();
    await expect(
      page.getByText(language === "cs" ? "○ Nedostupné" : "○ Unavailable", {
        exact: true,
      }),
    ).toHaveCount(1);
    const issue = page.getByRole("link", {
      name: language === "cs" ? /Nahlásit problém/ : /Report an issue/,
    });
    await expect(issue).toHaveAttribute("rel", "noopener noreferrer");
  });
}
