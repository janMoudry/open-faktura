import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

test.use({ actionTimeout: 15000, reducedMotion: "reduce" });

// Each test uses a disposable DB/server, never bootstraps the developer's account.
for (const english of [false, true])
  test(`financial dashboard balances and aging (${english ? "en" : "cs"})`, async ({
    page,
  }, info) => {
    test.setTimeout(90000);
    page.on("response", (response) => {
      if (
        response.status() >= 400 &&
        new URL(response.url()).pathname.startsWith("/api/")
      )
        console.log(
          "API response",
          response.status(),
          new URL(response.url()).pathname,
        );
    });
    const database = `company_e2e_${randomBytes(8).toString("hex")}`;
    const directory = mkdtempSync(join(tmpdir(), "faktura-auth-"));
    const secret = randomBytes(32).toString("hex");
    writeFileSync(join(directory, "bootstrap"), secret, { mode: 0o600 });
    const port = 3500 + info.workerIndex;
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
        .poll(
          async () =>
            fetch(`http://127.0.0.1:${port}/api/ready`)
              .then((r) => r.status)
              .catch(() => 0),
          { timeout: 30000 },
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
      const labels = JSON.parse(
        readFileSync(
          `apps/web/src/locales/${english ? "en" : "cs"}.json`,
          "utf8",
        ),
      );
      sql(
        `INSERT INTO received_invoice(company_id,number,data,total,paid,received_on,due_on) VALUES(${alpha.id},'DASH-P',jsonb_build_object('issued_on',current_date::text,'currency','CZK','lines',jsonb_build_array(jsonb_build_object('net','100.00','vat','21.00'))),121,21,current_date,current_date-31)`,
        database,
      );
      sql(
        `INSERT INTO sales_document(company_id,kind,creator_id,request_id,initial_data,data,number,advance_totals,advance_paid) SELECT ${alpha.id},'advance',user_id,'00000000-0000-4000-8000-000000000777','{}',jsonb_build_object('invoice',jsonb_build_object('issued_on',(current_date-1)::text,'due_on',(current_date-1)::text,'customer',jsonb_build_object('name','Synthetic customer'))),'DASH-ADV','{"currency":"CZK","payable":"50.00"}',10 FROM membership WHERE company_id=${alpha.id} LIMIT 1`,
        database,
      );
      sql(
        `UPDATE company SET settings=jsonb_set(settings,'{tax_mode}','"vat_payer"') WHERE id=${alpha.id}`,
        database,
      );
      sql(
        `WITH a AS (INSERT INTO cash_account(company_id,name,opening_on,opening_balance,currency) VALUES(${alpha.id},'Synthetic cash','2024-01-01',0,'CZK') RETURNING id) INSERT INTO cash_movement(company_id,account_id,number,data,issuer,amount,booked_on,tax) SELECT ${alpha.id},id,'CASH-TAX','{}','{}',121,current_date,jsonb_build_object('tax',jsonb_build_object('tax_on',current_date::text),'amounts',jsonb_build_object('net','100.00','vat','21.00')) FROM a`,
        database,
      );
      await page.goto(`/#/companies/${alpha.id}/overview`);
      await expect(
        page.getByRole("heading", { name: labels.dashboardTitle, exact: true }),
      ).toBeVisible();
      const dashboard = page.getByRole("region", {
        name: labels.dashboardTitle,
        exact: true,
      });
      await expect(
        dashboard.getByText(english ? "121.00 CZK" : "121,00 Kč", {
          exact: true,
        }),
      ).toBeVisible();
      await expect(
        dashboard.getByText("Synthetic customer", { exact: true }),
      ).toHaveCount(2);
      const payables = dashboard.locator("section").filter({
        has: page.getByRole("heading", {
          name: labels.dashboardPayables,
          exact: true,
        }),
      });
      await expect(
        payables
          .getByRole("row")
          .filter({ hasText: labels.dashboard_days_31_60 }),
      ).toContainText(english ? "100.00 CZK" : "100,00 Kč");
      await dashboard
        .getByRole("button", { name: new RegExp(labels.reportValueMode) })
        .click();
      await page
        .getByRole("option", { name: labels.reportNet, exact: true })
        .click();
      await expect(
        dashboard.getByText(english ? "121.00 CZK" : "121,00 Kč", {
          exact: true,
        }),
      ).toHaveCount(0);
      await dashboard
        .getByRole("button", { name: new RegExp(labels.dashboardAgingScope) })
        .click();
      await page
        .getByRole("option", { name: labels.dashboardAllYears, exact: true })
        .click();
      await expect(
        payables
          .getByRole("row")
          .filter({ hasText: labels.dashboard_days_31_60 }),
      ).toContainText(english ? "100.00 CZK" : "100,00 Kč");
      const tax = page.getByRole("region", {
        name: labels.taxDashboardTitle,
        exact: true,
      });
      await expect(
        tax.getByRole("heading", { name: labels.taxDashboardVat, exact: true }),
      ).toBeVisible();
      await expect(
        tax.getByText(english ? "21.00 CZK" : "21,00 Kč", { exact: true }),
      ).toHaveCount(4);
      await expect(
        tax.getByRole("row").filter({
          has: page.getByRole("link", { name: "CASH-TAX", exact: true }),
        }),
      ).toContainText(english ? "100.00 CZK" : "100,00 Kč");
      await tax
        .getByRole("button", { name: new RegExp(labels.taxDashboardGrouping) })
        .click();
      await page
        .getByRole("option", { name: labels.reportQuarterly, exact: true })
        .click();
      await expect(
        tax.getByText(
          new Date().getFullYear() +
            "-Q" +
            (Math.floor(new Date().getMonth() / 3) + 1),
          { exact: true },
        ),
      ).toBeVisible();
      await tax
        .getByRole("button", { name: new RegExp(labels.taxDashboardSources) })
        .click();
      await page
        .getByRole("option", { name: labels.taxDashboardVat, exact: true })
        .click();
      await expect(
        tax.getByRole("link", { name: "DASH-P", exact: true }),
      ).toHaveAttribute("href", `#/companies/${alpha.id}/received-invoices/1`);
      expect(
        (
          await new AxeBuilder({ page })
            .withTags(["wcag2a", "wcag2aa"])
            .analyze()
        ).violations,
      ).toEqual([]);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await expect(
        tax.getByRole("link", { name: "CASH-TAX", exact: true }),
      ).toHaveAttribute("href", `#/companies/${alpha.id}/cash/1`);
      await tax.screenshot({
        path: `docs/design/tax-dashboard-${english ? "en" : "cs"}-${info.project.name}.png`,
      });
      await page.screenshot({
        path: `docs/design/dashboard-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page.goto(`/#/companies/${beta.id}/overview`);
      await expect(
        tax.getByRole("heading", {
          name: labels.taxDashboardTurnover,
          exact: true,
        }),
      ).toBeVisible();
      await expect(
        tax.getByRole("heading", { name: labels.taxDashboardVat, exact: true }),
      ).toHaveCount(0);
      await expect(
        tax.getByRole("link", { name: "DASH-P", exact: true }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("heading", { name: labels.dashboardTitle, exact: true }),
      ).toBeVisible();
      await expect(
        page
          .getByRole("region", { name: labels.dashboardTitle, exact: true })
          .getByText(english ? "100.00 CZK" : "100,00 Kč", { exact: true }),
      ).toHaveCount(0);

      const foreign = JSON.parse(
        readFileSync("tests/fixtures/invoice-draft.json", "utf8"),
      );
      foreign.issued_on = new Date().toISOString().slice(0, 10);
      foreign.due_on = foreign.issued_on;
      foreign.tax_on = null;
      foreign.calculation.tax_mode = "non_payer";
      foreign.calculation.rounding = "none";
      foreign.calculation.discount_percent = "0";
      foreign.calculation.lines = foreign.calculation.lines.slice(0, 1);
      Object.assign(foreign.calculation.lines[0], {
        quantity: "1",
        unit_price: "100",
        discount_percent: "0",
        vat_percent: "0",
      });
      foreign.supplier.registration_number = "12345678";
      foreign.payment.method = "bank_transfer";
      foreign.payment.bank_account = "12345678/0100";
      const created = await page.request.post(
        `http://127.0.0.1:${port}/api/companies/${beta.id}/invoices`,
        {
          headers: {
            Origin: "http://127.0.0.1:5173",
            "X-Faktura-Request": "1",
          },
          data: { request_id: crypto.randomUUID(), data: foreign },
        },
      );
      expect(created.ok()).toBe(true);
      const draft = await created.json();
      await page.goto(`/#/companies/${beta.id}/invoices/${draft.id}`);
      await page
        .getByRole("button", { name: new RegExp(labels.documentCurrency) })
        .click();
      await page.getByRole("option", { name: "EUR", exact: true }).click();
      let rateFailure = false;
      await page.route("**/api/companies/*/exchange-rates?*", async (route) => {
        const url = new URL(route.request().url());
        await route.fulfill({
          status: rateFailure ? 503 : 200,
          contentType: "application/json",
          body: JSON.stringify(
            rateFailure
              ? { error: "unavailable" }
              : {
                  currency: "EUR",
                  requested_on: url.searchParams.get("date"),
                  published_on: url.searchParams.get("date"),
                  source: "CNB",
                  exchange_rate: { rate: "25", units: "1" },
                },
          ),
        });
      });
      await page
        .getByRole("button", { name: labels.exchangeLoad, exact: true })
        .click();
      await expect(
        page.getByLabel(labels.exchangeRate, { exact: true }),
      ).toHaveValue("25");
      await expect(
        page.getByLabel(labels.exchangeUnits, { exact: true }),
      ).toHaveValue("1");
      await expect(
        page.getByRole("status").filter({ hasText: /ČNB|CNB/ }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/exchange-rate-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      rateFailure = true;
      await page
        .getByRole("button", { name: labels.exchangeLoad, exact: true })
        .click();
      await expect(
        page.getByText(labels.exchangeFailed, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByLabel(labels.exchangeRate, { exact: true }),
      ).toHaveValue("25");
      await expect(
        page
          .getByText(english ? "100.00 EUR" : "100,00 EUR", { exact: true })
          .first(),
      ).toBeVisible();
      const saved = page.waitForResponse(
        (r) =>
          r.url().endsWith(`/invoices/${draft.id}`) &&
          r.request().method() === "PUT" &&
          r.ok(),
      );
      await page
        .getByRole("button", {
          name: english ? "Save draft" : "Uložit návrh",
          exact: true,
        })
        .click();
      await saved;
      await page.reload();
      await expect(
        page.getByLabel(labels.exchangeRate, { exact: true }),
      ).toHaveValue("25");
      await page
        .getByRole("button", {
          name: english ? "Issue invoice" : "Vystavit fakturu",
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: english ? "Issue invoice" : "Vystavit fakturu",
          exact: true,
        })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toContainText("FV");
      await page
        .getByRole("tab", { name: labels.paymentsTitle, exact: true })
        .click();
      await page
        .getByRole("button", { name: labels.paymentAdd, exact: true })
        .click();
      await page
        .getByLabel(english ? "Amount (EUR)" : "Částka (EUR)", { exact: true })
        .fill("40");
      await page.screenshot({
        path: `docs/design/fx-payment-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("dialog")
        .getByRole("button", { name: labels.paymentConfirm, exact: true })
        .click();
      await expect(page.getByRole("dialog")).toHaveCount(0);
      await page.goto(`/#/companies/${beta.id}/overview`);
      await expect(
        page
          .getByRole("region", { name: labels.dashboardTitle, exact: true })
          .getByText(english ? "1,500.00 CZK" : "1 500,00 Kč", { exact: true })
          .first(),
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
