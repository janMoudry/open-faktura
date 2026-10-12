import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

test.use({ actionTimeout: 15000 });

// Each test uses a disposable DB/server, never bootstraps the developer's account.
for (const english of [false, true])
  test(`catalog CRUD, snapshots and isolation (${english ? "en" : "cs"})`, async ({
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
          url: `http://127.0.0.1:${port}${url.pathname}${url.search}`,
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
      const text = (cs: string, en: string) => (english ? en : cs);
      if (english) {
        await page.getByRole("button", { name: "Uživatelské menu" }).click();
        await page.getByRole("button", { name: /Jazyk aplikace/ }).click();
        await page.getByRole("option", { name: /English/ }).click();
        await page.keyboard.press("Escape");
      }
      const settings = {
        name: "Alpha",
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
        tax_effective_from: "",
        currency: "CZK",
        timezone: "Europe/Prague",
        maturity_days: 14,
        payment_method: "cash",
        iban: "",
        number_prefix: "FV",
        number_year: 2026,
        next_number: 1,
      };
      const makeProfile = async (name: string) => {
        const response = await page.request.post(
          `http://127.0.0.1:${port}/api/companies`,
          {
            headers: {
              Origin: "http://127.0.0.1:5173",
              "X-Faktura-Request": "1",
            },
            data: {
              request_id: crypto.randomUUID(),
              settings: { ...settings, name },
            },
          },
        );
        expect(response.ok()).toBe(true);
        return response.json();
      };
      const alpha = await makeProfile("Alpha");
      const beta = await makeProfile("Beta");
      await page.goto(`/#/companies/${alpha.id}/catalog`);
      await expect(
        page.getByText(
          text("Ceník je zatím prázdný", "Your price list is empty"),
        ),
      ).toBeVisible();
      await page
        .getByRole("link", {
          name: text("Nová položka ceníku", "New price list item"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Název položky", "Item name"), { exact: true })
        .fill("Synthetic consulting");
      await page
        .getByLabel(text("Katalogové číslo", "Catalog code"), { exact: true })
        .fill("SERVICE-01");
      await page
        .getByLabel(text("Cena", "Price"), { exact: true })
        .fill("1250,25");
      await page
        .getByLabel(text("Prodávat po", "Sell in quantities of"), {
          exact: true,
        })
        .fill("0");
      await page
        .getByRole("button", {
          name: text("Uložit položku", "Save item"),
          exact: true,
        })
        .click();
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: text("označené", "highlighted") }),
      ).toContainText(text("Zkontrolujte", "Check"));
      await page
        .getByLabel(text("Prodávat po", "Sell in quantities of"), {
          exact: true,
        })
        .fill("2");
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/catalog-editor-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("button", {
          name: text("Uložit položku", "Save item"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Synthetic consulting",
      );
      const itemUrl = page.url();
      const itemId = itemUrl.split("/").pop()!;
      const headers = {
        Origin: "http://127.0.0.1:5173",
        "X-Faktura-Request": "1",
      };
      const endpoint = `http://127.0.0.1:${port}/api/companies/${alpha.id}/catalog/${itemId}`;
      await page.reload();
      await expect(page.getByText("SERVICE-01", { exact: true })).toBeVisible();
      const a11y = await new AxeBuilder({ page }).analyze();
      expect(
        a11y.violations.filter(
          (v) => v.impact === "critical" || v.impact === "serious",
        ),
      ).toEqual([]);
      await page
        .getByRole("button", { name: text("Upravit", "Edit"), exact: true })
        .click();
      await page
        .getByLabel(text("Cena", "Price"), { exact: true })
        .fill("1500");
      // Concurrent API editor wins; UI must preserve local input on conflict.
      const current = await (await page.request.get(endpoint)).json();
      expect(
        (
          await page.request.put(endpoint, {
            headers,
            data: {
              revision: current.revision,
              archived: false,
              data: { ...current.data, code: "SERVER" },
            },
          })
        ).ok(),
      ).toBe(true);
      await page
        .getByRole("button", {
          name: text("Uložit změny", "Save changes"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("alert")).toContainText(
        text("mezitím", "Someone else"),
      );
      await expect(
        page.getByLabel(text("Cena", "Price"), { exact: true }),
      ).toHaveValue("1500");
      // Full reload intentionally abandons this test's unsaved changes.
      page.once("dialog", (dialog) => dialog.accept());
      await page.reload();
      await page
        .getByRole("button", { name: text("Upravit", "Edit"), exact: true })
        .click();
      await page
        .getByLabel(text("Cena", "Price"), { exact: true })
        .fill("1500");
      await page
        .getByRole("button", {
          name: text("Uložit změny", "Save changes"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Položka byla uložena.", "Item saved.")),
      ).toBeVisible();
      await page.goto(`/#/companies/${alpha.id}/catalog`);
      await expect(
        page.getByRole("link", { name: "Synthetic consulting", exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/catalog-list-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("textbox", {
          name: text(
            "Hledat název, katalogové číslo nebo EAN",
            "Search name, catalog code or EAN",
          ),
        })
        .fill("no match");
      await page
        .getByRole("button", { name: text("Hledat", "Search"), exact: true })
        .click();
      await expect(
        page.getByText(text("Žádné odpovídající položky", "No matching items")),
      ).toBeVisible();
      await page.goto(`/#/companies/${beta.id}/catalog`);
      await expect(
        page.getByText(
          text("Ceník je zatím prázdný", "Your price list is empty"),
        ),
      ).toBeVisible();
      await page.goto(`/#/companies/${alpha.id}/invoices/new`);
      const picker = page.getByRole("combobox", {
        name: text("Přidat z ceníku", "Add from price list"),
        exact: true,
      });
      await picker.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: `docs/design/catalog-invoice-${english ? "en" : "cs"}-${info.project.name}.png`,
      });
      await picker.click();
      await expect(
        page.getByRole("option", { name: /Synthetic consulting/ }),
      ).toBeVisible();
      await expect(page.locator(".ui-select-popover")).toHaveCSS(
        "opacity",
        "1",
      );
      await page.screenshot({
        path: `docs/design/catalog-picker-${english ? "en" : "cs"}-${info.project.name}.png`,
      });
      await page.getByRole("option", { name: /Synthetic consulting/ }).click();
      await expect(
        page
          .getByLabel(text("Popis položky", "Item description"), {
            exact: true,
          })
          .last(),
      ).toHaveValue("Synthetic consulting");
      await page
        .getByRole("button", {
          name: text("Uložit návrh", "Save draft"),
          exact: true,
        })
        .click();
      await expect(page).toHaveURL(/invoices\/\d+$/);
      // Draft holds the copied price; changing the catalog cannot mutate it.
      const before = await (await page.request.get(endpoint)).json();
      expect(
        (
          await page.request.put(endpoint, {
            headers,
            data: {
              revision: before.revision,
              archived: false,
              data: { ...before.data, unit_price: "9999" },
            },
          })
        ).ok(),
      ).toBe(true);
      await page.reload();
      await expect(
        page
          .getByLabel(text("Jednotková cena", "Unit price"), { exact: true })
          .last(),
      ).toHaveValue("1500");
      page.once("dialog", (dialog) => dialog.accept());
      await page.goto(itemUrl);
      await page
        .getByRole("button", { name: text("Akce", "Actions"), exact: true })
        .click();
      await page
        .getByRole("menuitem", {
          name: text("Archivovat položku", "Archive item"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Archivovat položku", "Archive item"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Archivované", "Archived"), { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: text("Akce", "Actions"), exact: true })
        .click();
      await page
        .getByRole("menuitem", {
          name: text("Obnovit položku", "Restore item"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Obnovit položku", "Restore item"),
          exact: true,
        })
        .click();
      await expect(
        page.getByRole("button", {
          name: text("Upravit", "Edit"),
          exact: true,
        }),
      ).toBeVisible();
      sql(
        `UPDATE membership SET role='viewer' WHERE company_id=${alpha.id}`,
        database,
      );
      await page.reload();
      await expect(
        page.getByText(
          text(
            "Tento profil můžete pouze prohlížet.",
            "You have read-only access to this profile.",
          ),
        ),
      ).toBeVisible();
      await expect(
        page.getByRole("button", {
          name: text("Upravit", "Edit"),
          exact: true,
        }),
      ).toHaveCount(0);
    } finally {
      await page.unrouteAll({ behavior: "wait" }).catch(() => {});
      if (server && server.exitCode === null) {
        const exited = new Promise((resolve) => server!.once("exit", resolve));
        server.kill("SIGINT");
        await exited;
      }
      sql(`DROP DATABASE ${database} WITH (FORCE)`);
      rmSync(directory, { recursive: true, force: true });
    }
  });
