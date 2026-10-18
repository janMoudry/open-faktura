import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from "node:fs";
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
      const stockLabels = JSON.parse(
        readFileSync(
          `apps/web/src/locales/${english ? "en" : "cs"}.json`,
          "utf8",
        ),
      );
      await page.goto(`/#/companies/${alpha.id}/stock`);
      await page
        .getByRole("button", { name: new RegExp(stockLabels.stockItem) })
        .click();
      await page
        .getByRole("option", { name: "Synthetic consulting", exact: true })
        .click();
      await page
        .getByLabel(stockLabels.stockQuantity, { exact: true })
        .fill("10.1234");
      await page
        .getByLabel(stockLabels.stockManualCost, { exact: true })
        .fill("20");
      await page
        .getByLabel(stockLabels.stockReason, { exact: true })
        .fill("Synthetic opening");
      await page
        .getByRole("button", { name: stockLabels.stockOpen, exact: true })
        .click();
      await expect(
        page.getByText(new RegExp(stockLabels.stockBalance)),
      ).toContainText(english ? "10.1234" : "10,1234");
      await page
        .getByLabel(stockLabels.stockQuantity, { exact: true })
        .fill("-0.1234");
      await page
        .getByLabel(stockLabels.stockReason, { exact: true })
        .fill("Synthetic issue");
      await page
        .getByRole("button", { name: stockLabels.stockSave, exact: true })
        .click();
      await expect(
        page.getByText(new RegExp(stockLabels.stockBalance)),
      ).toContainText("10");
      await page
        .getByRole("button", { name: new RegExp(stockLabels.stockOperation) })
        .click();
      await page
        .getByRole("option", { name: stockLabels.stockCount, exact: true })
        .click();
      await page
        .getByLabel(stockLabels.stockQuantity, { exact: true })
        .fill("9.5");
      await page
        .getByLabel(stockLabels.stockReason, { exact: true })
        .fill("Synthetic count");
      await page
        .getByRole("button", { name: stockLabels.stockSave, exact: true })
        .click();
      await expect(
        page.getByText(new RegExp(stockLabels.stockBalance)),
      ).toContainText(english ? "9.5" : "9,5");
      const overview = page.getByRole("region", {
        name: stockLabels.stockOverviewTitle,
        exact: true,
      });
      await expect(
        overview.getByText(stockLabels.stockPartialValue, { exact: true }),
      ).toBeVisible();
      await overview
        .getByRole("button", {
          name: stockLabels.stockValuationDetail,
          exact: true,
        })
        .click();
      await overview
        .getByLabel(stockLabels.stockUnitCost, { exact: true })
        .fill("10.1234");
      await overview
        .getByLabel(stockLabels.stockCostReason, { exact: true })
        .fill("Synthetic stock cost");
      await overview
        .getByRole("button", { name: stockLabels.stockSaveCost, exact: true })
        .click();
      await expect(
        overview.getByText(stockLabels.stockPartialValue, { exact: true }),
      ).toHaveCount(0);
      await expect(
        overview.getByText(english ? /96.17/ : /96,17/).first(),
      ).toBeVisible();
      await overview
        .getByRole("button", {
          name: new RegExp(stockLabels.stockValuationMethod),
        })
        .click();
      await page
        .getByRole("option", {
          name: stockLabels.stockAutomaticMethod,
          exact: true,
        })
        .click();
      await expect(
        overview.getByText(english ? /190.00/ : /190,00/).first(),
      ).toBeVisible();
      await overview
        .getByRole("button", {
          name: new RegExp(stockLabels.stockValuationMethod),
        })
        .click();
      await page
        .getByRole("option", {
          name: stockLabels.stockManualMethod,
          exact: true,
        })
        .click();
      const statistics = page.getByRole("region", {
        name: stockLabels.stockStatistics,
        exact: true,
      });
      await expect(statistics).toBeVisible();
      await expect(
        statistics.locator("div").filter({
          has: page.getByText(stockLabels.stockStatClosing, { exact: true }),
        }),
      ).toContainText(english ? "9.5" : "9,5");
      await expect(
        statistics.locator("div").filter({
          has: page.getByText(stockLabels.stockStatIssued, { exact: true }),
        }),
      ).toContainText(english ? "0.1234" : "0,1234");

      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      expect(
        (
          await new AxeBuilder({ page })
            .withTags(["wcag2a", "wcag2aa"])
            .analyze()
        ).violations,
      ).toEqual([]);
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/stock-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page
        .getByRole("button", { name: stockLabels.accountMenu, exact: true })
        .click();
      await page
        .getByRole("button", { name: new RegExp(stockLabels.appearanceTheme) })
        .click();
      await page
        .getByRole("option", { name: stockLabels.appearanceDark, exact: true })
        .click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
      await page.reload();
      await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
      expect(
        (
          await new AxeBuilder({ page })
            .withTags(["wcag2a", "wcag2aa"])
            .analyze()
        ).violations,
      ).toEqual([]);
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/stock-dark-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page
        .getByRole("button", { name: stockLabels.accountMenu, exact: true })
        .click();
      await page
        .getByRole("button", { name: new RegExp(stockLabels.appearanceTheme) })
        .click();
      await page
        .getByRole("option", { name: stockLabels.appearanceLight, exact: true })
        .click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
      sql(
        `INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) VALUES(${alpha.id},'REPORT-SYNTHETIC','{"issued_on":"${new Date().getFullYear()}-01-15","lines":[{"net":"100.00","vat":"23.45"}]}',123.45,'${new Date().getFullYear()}-01-15','${new Date().getFullYear()}-01-30')`,
        database,
      );
      await page.goto(`/#/companies/${alpha.id}/reports`);
      await expect(
        page.getByRole("link", { name: "REPORT-SYNTHETIC" }),
      ).toBeVisible();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        stockLabels.navReports,
      );
      await expect(
        page.getByRole("region", {
          name: stockLabels.accountMenu,
          exact: true,
        }),
      ).toHaveCount(0);
      await page
        .getByRole("button", { name: new RegExp(stockLabels.reportValueMode) })
        .click();
      await page
        .getByRole("option", { name: stockLabels.reportNet, exact: true })
        .click();
      await expect(
        page.getByRole("row").filter({ hasText: "REPORT-SYNTHETIC" }),
      ).toContainText(english ? "100.00 CZK" : "100,00 Kč");
      for (const [grouping, label] of [
        ["quarter", stockLabels.reportQuarterly],
        ["year", stockLabels.reportYearly],
      ] as const) {
        await page
          .getByRole("button", { name: new RegExp(stockLabels.reportGrouping) })
          .click();
        const loaded = page.waitForResponse(
          (r) =>
            r.url().includes("/reports/period?") &&
            new URL(r.url()).searchParams.get("grouping") === grouping,
        );
        await page.getByRole("option", { name: label, exact: true }).click();
        const result = await loaded;
        expect(result.status()).toBe(200);
        const body = await result.json();
        expect(body.grouping).toBe(grouping);
        expect(body.months[0].month).toMatch(
          grouping === "quarter" ? /^\d{4}-Q1$/ : /^\d{4}$/,
        );
        await expect(
          page.getByRole("heading", { name: label, exact: true }),
        ).toBeVisible();
        await expect(
          page.getByRole("row").filter({ hasText: "REPORT-SYNTHETIC" }),
        ).toContainText(english ? "100.00 CZK" : "100,00 Kč");
      }
      await page
        .getByRole("button", { name: new RegExp(stockLabels.reportDisplay) })
        .click();
      await page
        .getByRole("option", {
          name: stockLabels.reportCumulative,
          exact: true,
        })
        .click();
      await page
        .getByRole("button", { name: new RegExp(stockLabels.reportCompare) })
        .click();
      await page
        .getByRole("option", {
          name: stockLabels.reportPreviousYear,
          exact: true,
        })
        .click();
      await expect(
        page.getByRole("button", {
          name: stockLabels.reportPreviousSources,
          exact: true,
        }),
      ).toBeVisible();
      const reportAxe = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa"])
        .analyze();
      expect(reportAxe.violations).toEqual([]);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await page.screenshot({
        path: `docs/design/report-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("button", {
          name: stockLabels.reportPreviousSources,
          exact: true,
        })
        .click();
      await expect(
        page.getByText(stockLabels.reportEmpty, { exact: true }),
      ).toBeVisible();
      await page.goto(itemUrl);

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
      await page
        .getByRole("button", {
          name: text("Přidat z ceníku", "Add from price list"),
          exact: true,
        })
        .click();
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
      const storedDraft = await (
        await page.request.get(
          `http://127.0.0.1:${port}/api/companies/${alpha.id}/invoices/${page.url().split("/").at(-1)}`,
        )
      ).json();
      expect(storedDraft.data.calculation.lines.at(-1).catalog_id).toBe(itemId);
      await expect(
        page.getByText(stockLabels.stockInvoiceLink, { exact: true }),
      ).toBeVisible();
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
      // Item sales uses issued snapshots, including catalog-linked services.
      const salesData = JSON.parse(
        readFileSync("tests/fixtures/invoice-draft.json", "utf8"),
      );
      salesData.supplier = {
        ...storedDraft.data.supplier,
        registration_number: "12345678",
      };
      salesData.payment.method = "cash";
      salesData.issued_on = new Date().toLocaleDateString("sv-SE");
      salesData.calculation.tax_mode = "non_payer";
      salesData.calculation.lines = [
        {
          ...salesData.calculation.lines[0],
          catalog_id: itemId,
          unit: "ks",
          quantity: "2",
          unit_price: "12.5",
          vat_percent: "0",
        },
      ];
      const salesDraftResponse = await page.request.post(
        `http://127.0.0.1:${port}/api/companies/${alpha.id}/invoices`,
        { headers, data: { request_id: crypto.randomUUID(), data: salesData } },
      );
      expect(salesDraftResponse.ok()).toBe(true);
      const salesDraft = await salesDraftResponse.json();
      const issuedResponse = await page.request.post(
        `http://127.0.0.1:${port}/api/companies/${alpha.id}/invoices/${salesDraft.id}/issue`,
        {
          headers,
          data: {
            request_id: crypto.randomUUID(),
            revision: salesDraft.revision,
          },
        },
      );
      expect(issuedResponse.ok()).toBe(true);
      const issuedSale = await issuedResponse.json();
      await page.goto(`/#/companies/${alpha.id}/item-sales`);
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        stockLabels.itemSalesTitle,
      );
      const itemSales = page.getByRole("region", {
        name: stockLabels.itemSalesTitle,
        exact: true,
      });
      await expect(
        itemSales
          .getByText(english ? "25.00 CZK" : "25,00 CZK", { exact: true })
          .first(),
      ).toBeVisible();
      await itemSales
        .getByRole("button", { name: "Synthetic consulting", exact: true })
        .click();
      await expect(
        page
          .getByRole("region", {
            name: stockLabels.itemSalesSources,
            exact: true,
          })
          .getByRole("link", { name: issuedSale.number }),
      ).toBeVisible();
      await page
        .getByRole("button", {
          name: stockLabels.itemSalesColumns,
          exact: true,
        })
        .click();
      await page
        .getByRole("checkbox", { name: stockLabels.itemSalesCode, exact: true })
        .uncheck();
      await expect(
        itemSales.getByRole("columnheader", {
          name: stockLabels.itemSalesCode,
          exact: true,
        }),
      ).toHaveCount(0);
      const downloadEvent = page.waitForEvent("download");
      await page.getByRole("button", { name: "CSV", exact: true }).click();
      expect((await downloadEvent).suggestedFilename()).toBe("item-sales.csv");
      await page.screenshot({
        path: `docs/design/item-sales-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await page
        .getByRole("button", { name: new RegExp(stockLabels.itemSalesKind) })
        .click();
      await page
        .getByRole("option", { name: stockLabels.navCreditNotes, exact: true })
        .click();
      await expect(
        page.getByText(stockLabels.itemSalesEmpty, { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: stockLabels.itemSalesClear, exact: true })
        .click();
      await expect(
        itemSales
          .getByText(english ? "25.00 CZK" : "25,00 CZK", { exact: true })
          .first(),
      ).toBeVisible();
      await page.goto(itemUrl);
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
