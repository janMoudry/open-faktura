import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

// Each test uses a disposable DB/server, never bootstraps the developer's account.
for (const english of [false, true])
  test(`billing profile persists and isolates settings (${english ? "en" : "cs"})`, async ({
    page,
  }, info) => {
    test.setTimeout(90000);
    const database = `company_e2e_${randomBytes(8).toString("hex")}`;
    const directory = mkdtempSync(join(tmpdir(), "faktura-auth-"));
    const secret = randomBytes(32).toString("hex");
    writeFileSync(join(directory, "bootstrap"), secret, { mode: 0o600 });
    const port = 3200 + info.workerIndex;
    const sql = (query: string, targetDatabase = "postgres") =>
      execFileSync(
        "docker",
        [
          "compose",
          "-f",
          "infra/compose.yaml",
          "exec",
          "-T",
          "postgres",
          "psql",
          "-U",
          "faktura",
          "-d",
          targetDatabase,
          "-v",
          "ON_ERROR_STOP=1",
          "-c",
          query,
        ],
        { stdio: "pipe" },
      );
    let server: ChildProcess | undefined;
    sql(`CREATE DATABASE ${database}`);
    try {
      server = spawn("target/debug/faktura-server", [], {
        env: {
          ...process.env,
          APP_ENV: "development",
          DATABASE_URL: `postgres://faktura:faktura@127.0.0.1:54329/${database}`,
          DATABASE_URL_FILE: undefined,
          API_ADDRESS: `127.0.0.1:${port}`,
          BOOTSTRAP_SECRET_FILE: join(directory, "bootstrap"),
          CORS_ORIGINS: "http://127.0.0.1:5173",
        },
        stdio: "ignore",
      });
      await expect
        .poll(async () =>
          fetch(`http://127.0.0.1:${port}/api/ready`)
            .then((r) => r.status)
            .catch(() => 0),
        )
        .toBe(200);
      await page.route("**/api/**", async (route) => {
        const url = new URL(route.request().url());
        await route.continue({
          url: `http://127.0.0.1:${port}${url.pathname}`,
        });
      });
      await page.waitForLoadState("networkidle");
      await page.goto("/");
      await page
        .getByLabel("E-mail", { exact: true })
        .fill("admin@example.test");
      await page
        .getByLabel("Heslo", { exact: true })
        .fill("Synthetic password 123!");
      await page.getByLabel("Zopakujte heslo").fill("Synthetic password 123!");
      await page.getByLabel("Instalační klíč", { exact: true }).fill(secret);
      await page.getByRole("button", { name: "Založit správce" }).click();
      await expect(page.getByRole("status")).toContainText(
        "Správce byl založen",
      );
      await page.getByLabel("Heslo", { exact: true }).fill("wrong password");
      await page.getByRole("button", { name: "Přihlásit se" }).click();
      await expect(page.getByRole("alert")).toContainText("nejsou platné");
      await page
        .getByLabel("Heslo", { exact: true })
        .fill("Synthetic password 123!");
      await page.getByRole("button", { name: "Přihlásit se" }).click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Fakturační profily",
      );
      if (english) {
        await page
          .getByRole("button", { name: /Uživatelské menu|Account menu/ })
          .click();
        await page.getByRole("button", { name: /Jazyk aplikace/ }).click();
        await page.getByRole("option", { name: /English/ }).click();
        await page.keyboard.press("Escape");
      }
      const text = (cs: string, en: string) => (english ? en : cs);
      await page
        .getByRole("button", {
          name: text("Přidat fakturační profil", "Add billing profile"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Jméno nebo název", "Name or business name"), {
          exact: true,
        })
        .fill("Synthetic Alpha");
      await page.route("**/contacts/ares/*", (route) =>
        route.fulfill({
          json: {
            registration_number: "00006947",
            name: "ARES name",
            vat_number: "CZ00006947",
            billing_address: {
              street: "Letenská 525/15",
              city: "Praha",
              postal_code: "11800",
              country: "CZ",
            },
          },
        }),
      );
      await page
        .getByLabel(text("IČO", "Registration number"), { exact: true })
        .fill("00006947");
      await page
        .getByLabel(text("IČO", "Registration number"), { exact: true })
        .press("Tab");
      await expect(
        page.getByLabel(text("Ulice a číslo", "Street and number")),
      ).toHaveValue("Letenská 525/15");
      await expect(
        page.getByLabel(text("Jméno nebo název", "Name or business name"), {
          exact: true,
        }),
      ).toHaveValue("Synthetic Alpha");
      await page.screenshot({
        path: `docs/design/profile-ares-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page
        .getByLabel(text("Ulice a číslo", "Street and number"))
        .fill("Test 1");
      await page
        .getByLabel(text("Město", "City"), { exact: true })
        .fill("Praha");
      await page
        .getByLabel(text("PSČ", "Postal code"), { exact: true })
        .fill("11000");
      await expect(
        page.getByLabel(text("Následující číslo", "Next number")),
      ).not.toBeVisible();
      await page
        .getByRole("button", {
          name: text("Uložit rozpracované", "Save progress"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Nastavení bylo uloženo.", "Settings saved."), {
          exact: true,
        }),
      ).toBeVisible();
      const firstUrl = page.url();
      const firstId = firstUrl.split("/").at(-1)!;
      await page.waitForLoadState("networkidle");
      await page.reload();
      await expect(
        page.getByLabel(text("Jméno nebo název", "Name or business name"), {
          exact: true,
        }),
      ).toHaveValue("Synthetic Alpha");
      // Incomplete submission opens the section containing the server validation error.
      await page
        .getByRole("button", {
          name: text("Uložit fakturační údaje", "Save billing details"),
          exact: true,
        })
        .click();
      await expect(
        page.getByRole("button", {
          name: text("Fakturační nastavení", "Billing preferences"),
          exact: true,
        }),
      ).toHaveAttribute("aria-pressed", "true");
      await page
        .getByRole("button", {
          name: text("Fakturační nastavení", "Billing preferences"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: english ? /^Open calendar:/ : /^Otevřít kalendář:/,
        })
        .click();
      await page.locator('[data-today="true"]').click();
      await page
        .getByRole("button", {
          name: text("Uložit fakturační údaje", "Save billing details"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(
          text("Údaje máte připravené.", "Your details are ready."),
          { exact: false },
        ),
      ).toBeVisible();
      await page.waitForLoadState("networkidle");
      await page.goto("/");
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Synthetic Alpha",
      );
      await page
        .getByRole("button", {
          name: text("Upravit nastavení", "Edit settings"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Jméno nebo název", "Name or business name"), {
          exact: true,
        })
        .fill("Synthetic Alpha Updated");
      await page
        .getByRole("button", {
          name: text("Uložit rozpracované", "Save progress"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Nastavení bylo uloženo.", "Settings saved."), {
          exact: true,
        }),
      ).toBeVisible();
      await page.waitForLoadState("networkidle");
      await page.reload();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Synthetic Alpha Updated",
      );
      await page
        .getByRole("button", { name: /Synthetic Alpha Updated/ })
        .click();
      await page
        .getByRole("button", {
          name: text("Přidat fakturační profil", "Add billing profile"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Jméno nebo název", "Name or business name"), {
          exact: true,
        })
        .fill("Synthetic Beta");
      await page
        .getByRole("button", {
          name: text("Fakturuji jako", "Invoice as"),
          exact: false,
        })
        .click();
      await page
        .getByRole("button", { name: "Synthetic Alpha Updated", exact: true })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Zůstat zde", "Stay here"),
          exact: true,
        })
        .click();
      await expect(
        page.getByLabel(text("Jméno nebo název", "Name or business name"), {
          exact: true,
        }),
      ).toHaveValue("Synthetic Beta");

      await page
        .getByRole("button", {
          name: text("Uložit rozpracované", "Save progress"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Nastavení bylo uloženo.", "Settings saved."), {
          exact: true,
        }),
      ).toBeVisible();
      await page.waitForLoadState("networkidle");
      await page.goto(firstUrl);
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Synthetic Alpha Updated",
      );
      // Multiple billing identities remain available and the choice survives returning home.
      await expect(
        page.getByRole("link", {
          name: text("Nová faktura", "New invoice"),
          exact: true,
        }),
      ).toHaveAttribute("href", /companies\/\d+\/invoices\/new$/);
      await page
        .getByRole("button", { name: /Synthetic Alpha Updated/ })
        .click();
      await page
        .getByRole("button", { name: "Synthetic Beta", exact: true })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Synthetic Beta",
      );
      await page.waitForLoadState("networkidle");
      await page.goto("/");
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Synthetic Beta",
      );
      await page.getByRole("button", { name: /Synthetic Beta/ }).click();
      await page
        .getByRole("button", { name: "Synthetic Alpha Updated", exact: true })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Synthetic Alpha Updated",
      );
      await page.goto("/#/companies");
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        text("Fakturační profily", "Billing profiles"),
      );
      await page.screenshot({
        path: `docs/design/profiles-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("button", {
          name: text(
            "Smazat profil Synthetic Beta",
            "Delete profile Synthetic Beta",
          ),
          exact: true,
        })
        .click();
      const deletionDialog = page.getByRole("dialog");
      await expect(
        deletionDialog.getByRole("button", {
          name: text("Smazat profil", "Delete profile"),
          exact: true,
        }),
      ).toBeDisabled();
      await deletionDialog
        .getByRole("button", { name: text("Zrušit", "Cancel"), exact: true })
        .click();
      await expect(
        page.getByRole("link", { name: "Synthetic Beta", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", {
          name: text(
            "Smazat profil Synthetic Beta",
            "Delete profile Synthetic Beta",
          ),
          exact: true,
        })
        .click();
      await deletionDialog
        .getByLabel(
          text("Název profilu pro potvrzení", "Profile name to confirm"),
        )
        .fill("Synthetic Beta");
      await deletionDialog
        .getByRole("button", {
          name: text("Smazat profil", "Delete profile"),
          exact: true,
        })
        .click();
      await expect(deletionDialog).toHaveCount(0);
      await expect(
        page.getByRole("link", { name: "Synthetic Beta", exact: true }),
      ).toHaveCount(0);
      await page
        .getByRole("link", { name: "Synthetic Alpha Updated", exact: true })
        .click();
      // Another account owns a company; installation admin does not bypass membership.
      sql(
        "INSERT INTO app_user(email,password_hash) SELECT 'other@example.test',password_hash FROM app_user WHERE email='admin@example.test'",
        database,
      );
      const headers = {
        "Content-Type": "application/json",
        "X-Faktura-Request": "1",
        Origin: "http://127.0.0.1:5173",
      };
      const otherLogin = await fetch(
        `http://127.0.0.1:${port}/api/auth/login`,
        {
          method: "POST",
          headers,
          body: JSON.stringify({
            email: "other@example.test",
            password: "Synthetic password 123!",
          }),
        },
      );
      const otherCookie = otherLogin.headers.get("set-cookie")!.split(";")[0];
      const hiddenSettings = {
        name: "Hidden Company",
        entity_type: "individual",
        registration_number: "",
        vat_number: "",
        street: "Test 1",
        city: "Praha",
        postal_code: "11000",
        country: "CZ",
        email: "",
        phone: "",
        tax_mode: "non_payer",
        tax_effective_from: "2026-01-01",
        currency: "CZK",
        timezone: "Europe/Prague",
        maturity_days: 14,
        payment_method: "cash",
        iban: "",
        number_prefix: "FV",
        number_year: 2026,
        next_number: 1,
      };
      const hiddenResponse = await fetch(
        `http://127.0.0.1:${port}/api/companies`,
        {
          method: "POST",
          headers: { ...headers, Cookie: otherCookie },
          body: JSON.stringify({
            request_id: "00000000-0000-4000-8000-000000000007",
            settings: hiddenSettings,
          }),
        },
      );
      const hidden = await hiddenResponse.json();
      expect(hiddenResponse.status).toBe(200);
      await page.waitForLoadState("networkidle");
      await page.goto(`/#/companies/${hidden.id}`);
      await expect(page.getByRole("alert")).toContainText(
        text("Firma neexistuje", "The company does not exist"),
      );
      await expect(
        page.getByText("Hidden Company", { exact: true }),
      ).toHaveCount(0);
      sql(
        `INSERT INTO membership(company_id,user_id,role) SELECT ${Number(hidden.id)},id,'viewer' FROM app_user WHERE email='admin@example.test'`,
        database,
      );
      await page.waitForLoadState("networkidle");
      await page.reload();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Hidden Company",
      );
      await expect(
        page.getByRole("button", {
          name: text("Upravit nastavení", "Edit settings"),
          exact: true,
        }),
      ).toHaveCount(0);
      const denied = await page.request.put(
        `http://127.0.0.1:${port}/api/companies/${hidden.id}`,
        {
          headers,
          data: {
            revision: hidden.revision,
            step: 0,
            complete: false,
            settings: hiddenSettings,
          },
        },
      );
      expect(denied.status()).toBe(403);
      await page.waitForLoadState("networkidle");
      await page.goto(firstUrl);
      await page
        .getByRole("button", {
          name: text("Upravit nastavení", "Edit settings"),
          exact: true,
        })
        .click();
      // A concurrent tab wins; saving this form must surface a conflict, not overwrite.
      const current = await (
        await page.request.get(
          `http://127.0.0.1:${port}/api/companies/${firstId}`,
        )
      ).json();
      const changed = await page.request.put(
        `http://127.0.0.1:${port}/api/companies/${firstId}`,
        {
          headers,
          data: {
            revision: current.revision,
            step: 0,
            complete: false,
            settings: { ...current.settings, name: "Concurrent change" },
          },
        },
      );
      expect(changed.status()).toBe(200);
      await page
        .getByRole("button", {
          name: text("Uložit rozpracované", "Save progress"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("alert")).toContainText(
        text("Nastavení mezitím někdo změnil", "The settings changed"),
      );
      await page
        .getByRole("button", {
          name: text("Načíst aktuální verzi", "Load current version"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Concurrent change",
      );
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
      await page
        .getByRole("button", {
          name: text("Upravit nastavení", "Edit settings"),
          exact: true,
        })
        .click();
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/p03-company-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      // Direct inaccessible IDs must never retain the previous company's content.
      await page.waitForLoadState("networkidle");
      await page.goto("/#/companies/9999999");
      await expect(page.getByRole("alert")).toContainText(
        text("Firma neexistuje", "The company does not exist"),
      );
      await expect(
        page.getByText("Synthetic Alpha Updated", { exact: true }),
      ).toHaveCount(0);
      if (english) {
        await page
          .getByRole("button", { name: /Uživatelské menu|Account menu/ })
          .click();
        await page.getByRole("button", { name: /App language/ }).click();
        await page.getByRole("option", { name: /Čeština/ }).click();
        await page.keyboard.press("Escape");
      }
      expect(await page.evaluate(() => document.cookie)).not.toContain(
        "faktura_session",
      );
      await page.getByRole("button", { name: "Uživatelské menu" }).click();
      await expect(
        page.getByText("admin@example.test", { exact: true }),
      ).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(
        page.getByRole("button", { name: "Uživatelské menu" }),
      ).toBeFocused();
      await page.getByRole("button", { name: "Uživatelské menu" }).click();
      await page.screenshot({
        path: `docs/design/account-menu-${info.project.name}-${english ? "en" : "cs"}.png`,
        animations: "disabled",
      });
      if (english) {
        await page.getByRole("button", { name: /Jazyk aplikace/ }).click();
        await page.getByRole("option", { name: /English/ }).click();
      }
      await page
        .getByRole("button", {
          name: english ? "Account settings" : "Nastavení účtu",
          exact: true,
        })
        .click();
      const dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(english ? "Name" : "Jméno", { exact: true })
        .fill("Jan Test");
      await dialog
        .getByRole("button", {
          name: english ? "Save changes" : "Uložit změny",
        })
        .click();
      await expect(dialog.getByRole("status")).toContainText(
        english ? "Account settings saved" : "Nastavení účtu bylo uloženo",
      );
      await dialog
        .getByLabel(english ? "Email" : "E-mail", { exact: true })
        .fill("new@example.test");
      await dialog
        .getByLabel(english ? "Current password" : "Současné heslo", {
          exact: true,
        })
        .fill("wrong password");
      await dialog
        .getByRole("button", {
          name: english ? "Save changes" : "Uložit změny",
        })
        .click();
      await expect(dialog.getByRole("alert")).toContainText(
        english
          ? "current password is incorrect"
          : "Současné heslo není správné",
      );
      await expect(
        dialog.getByLabel(english ? "Email" : "E-mail", { exact: true }),
      ).toHaveValue("new@example.test");
      await dialog
        .getByLabel(english ? "Current password" : "Současné heslo", {
          exact: true,
        })
        .fill("Synthetic password 123!");
      await dialog
        .getByRole("button", {
          name: english ? "Change password" : "Změna hesla",
        })
        .click();
      await dialog
        .getByLabel(english ? "New password" : "Nové heslo", { exact: true })
        .fill("Changed password 123!");
      await dialog
        .getByLabel(english ? "Confirm password" : "Zopakujte heslo", {
          exact: true,
        })
        .fill("Changed password 123!");
      await dialog
        .getByRole("button", {
          name: english ? "Save changes" : "Uložit změny",
        })
        .click();
      await expect(dialog.getByRole("status")).toBeVisible();
      await expect(
        dialog.getByLabel(english ? "New password" : "Nové heslo", {
          exact: true,
        }),
      ).toHaveCount(0);
      await page.screenshot({
        path: `docs/design/account-settings-${info.project.name}-${english ? "en" : "cs"}.png`,
        animations: "disabled",
      });
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page.keyboard.press("Escape");
      await expect(dialog).toHaveCount(0);
      await page.reload();
      await page
        .getByRole("button", {
          name: english ? "Account menu" : "Uživatelské menu",
        })
        .click();
      await expect(page.getByText("Jan Test", { exact: true })).toBeVisible();
      await expect(
        page.getByText("new@example.test", { exact: true }),
      ).toBeVisible();
      if (english) {
        await page.getByRole("button", { name: /App language/ }).click();
        await page.getByRole("option", { name: /Čeština/ }).click();
      }
      await page.getByRole("button", { name: "Odhlásit se" }).click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Přihlášení",
      );
      await page.waitForLoadState("networkidle");
      await page.goto("/#/setup");
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Přihlášení",
      );
      await expect(
        page.getByLabel("Instalační klíč", { exact: true }),
      ).toHaveCount(0);
      await page.getByLabel("E-mail", { exact: true }).fill("new@example.test");
      await page
        .getByLabel("Heslo", { exact: true })
        .fill("Changed password 123!");
      await page
        .getByRole("button", { name: "Přihlásit se", exact: true })
        .click();
      await expect(
        page.getByRole("button", { name: "Uživatelské menu" }),
      ).toBeVisible();
    } finally {
      await page.unrouteAll({ behavior: "wait" });
      if (server && server.exitCode === null) {
        const exited = new Promise((resolve) => server!.once("exit", resolve));
        server.kill("SIGINT");
        await exited;
      }
      sql(`DROP DATABASE ${database} WITH (FORCE)`);
      rmSync(directory, { recursive: true, force: true });
    }
  });
