import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

test.use({ actionTimeout: 15000, reducedMotion: "reduce" });

// Each test uses a disposable DB/server, never bootstraps the developer's account.
for (const english of [false, true])
  test(`import previews, history and payments remain isolated (${english ? "en" : "cs"})`, async ({
    page,
  }, info) => {
    test.setTimeout(120000);
    const database = `company_e2e_${randomBytes(8).toString("hex")}`;
    const directory = mkdtempSync(join(tmpdir(), "faktura-auth-"));
    const secret = randomBytes(32).toString("hex");
    writeFileSync(join(directory, "bootstrap"), secret, { mode: 0o600 });
    const port = 3400 + info.workerIndex;
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
      const importUrl = `/#/companies/${alpha.id}/imports`;
      await page.goto(importUrl);
      await page
        .getByLabel(
          text("Identifikátor zdrojové agendy", "Source account identifier"),
          { exact: true },
        )
        .fill("synthetic-idoklad-account");
      await page
        .getByLabel(text("Soubor k importu", "Import file"), { exact: true })
        .setInputFiles("tests/fixtures/import-records.json");
      await page
        .getByRole("button", {
          name: text("Zkontrolovat import", "Check import"),
          exact: true,
        })
        .click();
      await expect(
        page.getByRole("heading", {
          name: text("3. Náhled a výsledek", "3. Preview and results"),
        }),
      ).toBeVisible();
      await expect(
        page.getByText(text("Připraveno", "Ready"), { exact: true }),
      ).toHaveCount(2);
      await expect(
        page.getByText(text("Cílový profil: Alpha", "Target profile: Alpha"), {
          exact: true,
        }),
      ).toBeVisible();
      const zero = await page.request.get(
        `http://127.0.0.1:${port}/api/companies/${alpha.id}/invoices`,
      );
      expect((await zero.json()).total).toBe(0);
      await page.reload();
      await page
        .getByRole("button", { name: /synthetic-idoklad-account/ })
        .click();
      await page
        .getByRole("checkbox", {
          name: english ? /I reviewed/ : /Zkontroloval/,
        })
        .check();
      await page
        .getByRole("button", {
          name: text(
            "Potvrdit a pokračovat v importu",
            "Confirm and continue import",
          ),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Importováno", "Imported"), { exact: true }),
      ).toHaveCount(2);
      await expect(page.getByRole("status")).toContainText(
        text("Dávka dokončena", "Batch complete"),
      );
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page.evaluate(() => window.scrollTo(0, 0));
      await expect(page.getByRole("status")).not.toContainText("{count}");
      await page.screenshot({
        path: `docs/design/import-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      const downloadPromise = page.waitForEvent("download");
      await page
        .getByRole("button", {
          name: text("Stáhnout report JSON", "Download JSON report"),
        })
        .click();
      const download = await downloadPromise;
      expect(download.suggestedFilename()).toMatch(/^import-.*\.json$/);
      await page
        .getByRole("link", { name: "HIST-2025-001", exact: true })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toContainText(
        "HIST-2025-001",
      );
      await expect(
        page.getByText(english ? /Historical import:/ : /Historický import:/),
      ).toBeVisible();
      await expect(
        page.getByRole("button", {
          name: text("Stáhnout PDF", "Download PDF"),
          exact: true,
        }),
      ).toHaveCount(0);
      await page.goto(importUrl);
      await page
        .getByLabel(
          text("Identifikátor zdrojové agendy", "Source account identifier"),
          { exact: true },
        )
        .fill("synthetic-idoklad-account");
      await page
        .getByLabel(text("Soubor k importu", "Import file"), { exact: true })
        .setInputFiles("tests/fixtures/import-records.json");
      await page
        .getByRole("button", {
          name: text("Zkontrolovat import", "Check import"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Již přeneseno", "Already transferred"), {
          exact: true,
        }),
      ).toHaveCount(2);
      await page
        .getByRole("button", {
          name: text("Nový import", "New import"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: english ? /Data source/ : /Zdroj dat/,
        })
        .click();
      await page
        .getByRole("option", { name: "FakturaOnline", exact: true })
        .click();
      await page
        .getByLabel(
          text("Identifikátor zdrojové agendy", "Source account identifier"),
          { exact: true },
        )
        .fill("synthetic-fakturaonline-account");
      await page
        .getByLabel(text("Soubor k importu", "Import file"), { exact: true })
        .setInputFiles("tests/fixtures/import-contacts.xlsx");
      await expect(
        page.getByRole("heading", {
          name: text("2. Mapování kontaktů", "2. Contact mapping"),
        }),
      ).toBeVisible();
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page
        .getByRole("button", {
          name: text("Zkontrolovat import", "Check import"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Připraveno", "Ready"), { exact: true }),
      ).toHaveCount(1);
      await page
        .getByRole("checkbox", {
          name: english ? /I reviewed/ : /Zkontroloval/,
        })
        .check();
      await page
        .getByRole("button", {
          name: text(
            "Potvrdit a pokračovat v importu",
            "Confirm and continue import",
          ),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Importováno", "Imported"), { exact: true }),
      ).toHaveCount(1);
      await page
        .getByRole("button", {
          name: text("Nový import", "New import"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", { name: english ? /Data source/ : /Zdroj dat/ })
        .click();
      await page
        .getByRole("option", { name: "Money S3 (XML)", exact: true })
        .click();
      await page
        .getByLabel(
          text("Identifikátor zdrojové agendy", "Source account identifier"),
          { exact: true },
        )
        .fill("synthetic-money");
      await page
        .getByRole("checkbox", {
          name: english
            ? /I confirm that the source Money/
            : /Potvrzuji, že domácí měna/,
        })
        .check();
      await page
        .getByLabel(text("Soubor k importu", "Import file"), { exact: true })
        .setInputFiles("tests/fixtures/import/money-s3-synthetic.xml");
      await expect(
        page.getByText("Synthetic item & service", { exact: true }),
      ).toBeVisible();
      await expect(page.getByText(/SYN-1.*750 CZK/)).toBeVisible();
      await page.screenshot({
        path: `docs/design/money-import-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("checkbox", {
          name: english ? /I reviewed/ : /Zkontroloval/,
        })
        .check();
      await page
        .getByRole("button", {
          name: text(
            "Potvrdit a pokračovat v importu",
            "Confirm and continue import",
          ),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Importováno", "Imported"), { exact: true }),
      ).toHaveCount(1);
      await expect(
        page.getByRole("link", {
          name: "Synthetic item & service",
          exact: true,
        }),
      ).toHaveAttribute("href", /catalog\/\d+$/);
      await page
        .getByRole("button", {
          name: text("Nový import", "New import"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", { name: english ? /Data source/ : /Zdroj dat/ })
        .click();
      await page
        .getByRole("option", {
          name: text("Jiný zdroj", "Other source"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: text("Typ tabulkových dat", "Table data type"),
        })
        .click();
      await page
        .getByRole("option", { name: text("Ceník", "Catalog"), exact: true })
        .click();
      await page
        .getByLabel(
          text("Identifikátor zdrojové agendy", "Source account identifier"),
          { exact: true },
        )
        .fill("synthetic-mapped-catalog");
      await page
        .getByRole("button", {
          name: text("Způsob zadání cen v souboru", "Price mode in the file"),
        })
        .click();
      await page
        .getByRole("option", {
          name: text("Bez DPH", "Net amount"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: text(
            "Desetinný oddělovač v souboru",
            "Decimal separator in the file",
          ),
        })
        .click();
      await page
        .getByRole("option", {
          name: text("Čárka (1250,50)", "Comma (1250,50)"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Soubor k importu", "Import file"), { exact: true })
        .setInputFiles("tests/fixtures/import/catalog-synthetic.csv");
      await expect(
        page.getByRole("heading", {
          name: text("2. Mapování ceníku", "2. Catalog mapping"),
        }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/catalog-mapping-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("button", {
          name: text("Zkontrolovat import", "Check import"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText("Synthetic mapped service", { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByText(text("Připraveno", "Ready"), { exact: true }),
      ).toHaveCount(1);
      await page.screenshot({
        path: `docs/design/catalog-import-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("checkbox", {
          name: english ? /I reviewed/ : /Zkontroloval/,
        })
        .check();
      await page
        .getByRole("button", {
          name: text(
            "Potvrdit a pokračovat v importu",
            "Confirm and continue import",
          ),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Importováno", "Imported"), { exact: true }),
      ).toHaveCount(1);
      await expect(
        page.getByRole("link", {
          name: "Synthetic mapped service",
          exact: true,
        }),
      ).toHaveAttribute("href", /catalog\/\d+$/);
      await page.goto(`/#/companies/${beta.id}/imports`);
      await expect(
        page.getByText(
          text("Zatím tu nejsou žádné importy.", "No imports yet."),
          { exact: true },
        ),
      ).toBeVisible();
      sql(
        `UPDATE membership SET role='viewer' WHERE company_id=${Number(alpha.id)}`,
        database,
      );
      await page.goto(importUrl);
      await expect(
        page.getByLabel(text("Soubor k importu", "Import file"), {
          exact: true,
        }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("button", { name: /synthetic-fakturaonline-account/ }),
      ).toBeVisible();
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
