import { createServer } from "node:net";
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
  test(`received invoices (${english ? "en" : "cs"})`, async ({
    page,
    browser,
  }, info) => {
    test.setTimeout(120000);
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
    const messages: string[] = [];
    const smtp = createServer((socket) => {
      socket.write("220 capture ESMTP\r\n");
      let buffer = "",
        message = "",
        data = false;
      socket.on("data", (chunk) => {
        buffer += chunk.toString();
        while (buffer.includes("\r\n")) {
          const end = buffer.indexOf("\r\n");
          const line = buffer.slice(0, end);
          buffer = buffer.slice(end + 2);
          if (data) {
            if (line === ".") {
              messages.push(message);
              data = false;
              socket.write("250 accepted\r\n");
            } else message += line + "\r\n";
            continue;
          }
          if (line.startsWith("DATA")) {
            data = true;
            socket.write("354 proceed\r\n");
          } else if (line.startsWith("QUIT")) {
            socket.end("221 bye\r\n");
          } else socket.write("250 capture\r\n");
        }
      });
    });
    await new Promise<void>((resolve) => smtp.listen(0, "127.0.0.1", resolve));
    const smtpAddress = smtp.address();
    if (!smtpAddress || typeof smtpAddress === "string")
      throw new Error("SMTP capture");
    let server: ChildProcess | undefined;
    sql(`CREATE DATABASE ${database}`);
    try {
      server = spawn("target/debug/faktura-server", [], {
        env: {
          ...process.env,
          APP_ENV: "development",
          PUBLIC_WEB_URL: "http://127.0.0.1:5173",
          SMTP_HOST: "127.0.0.1",
          SMTP_PORT: String(smtpAddress.port),
          SMTP_SECURITY: "plain",
          SMTP_FROM: "sender@example.test",
          SMTP_USERNAME: undefined,
          SMTP_PASSWORD_FILE: undefined,
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
          { timeout: 20000 },
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
      const makeProfile = async (name: string, vat = false) => {
        const response = await page.request.post(
          `http://127.0.0.1:${port}/api/companies`,
          {
            headers: {
              Origin: "http://127.0.0.1:5173",
              "X-Faktura-Request": "1",
            },
            data: {
              request_id: crypto.randomUUID(),
              settings: {
                ...settings,
                name,
                ...(vat
                  ? {
                      tax_mode: "vat_payer",
                      vat_number: "CZ12345678",
                      registration_number: "12345678",
                    }
                  : {}),
              },
            },
          },
        );
        expect(response.ok()).toBe(true);
        return response.json();
      };
      const alpha = await makeProfile("Alpha", true),
        _beta = await makeProfile("Beta");
      const labels = JSON.parse(
        readFileSync(
          `apps/web/src/locales/${english ? "en" : "cs"}.json`,
          "utf8",
        ),
      );
      const stockHeaders = {
        Origin: "http://127.0.0.1:5173",
        "X-Faktura-Request": "1",
      };
      const stockBase = `http://127.0.0.1:${port}/api/companies/${alpha.id}`;
      const stockCreated = await page.request.post(`${stockBase}/catalog`, {
        headers: stockHeaders,
        data: {
          request_id: crypto.randomUUID(),
          data: {
            name: "Physical goods",
            code: "GOODS",
            barcode: "",
            unit: "ks",
            quantity: "1",
            unit_price: "100",
            currency: "CZK",
            price_mode: "net",
            vat_percent: "21",
          },
        },
      });
      expect(stockCreated.ok()).toBe(true);
      const stockItem = await stockCreated.json();
      expect(
        (
          await page.request.post(`${stockBase}/stock/commands`, {
            headers: stockHeaders,
            data: {
              request_id: crypto.randomUUID(),
              catalog_id: stockItem.id,
              action: "open",
              quantity: "0",
              booked_on: "2026-10-01",
              reason: "Synthetic opening",
            },
          })
        ).ok(),
      ).toBe(true);
      await page.goto(`/#/companies/${alpha.id}/inbox`);
      await page.getByLabel(labels.inboxUpload, { exact: true }).setInputFiles({
        name: "synthetic-inbox.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4\nSynthetic inbox test"),
      });
      await expect(
        page.getByRole("link", { name: "synthetic-inbox.pdf", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("link", { name: labels.inboxCreate, exact: true })
        .click();
      await page
        .getByLabel(labels.receivedSupplierName, { exact: true })
        .fill("Synthetic vendor");
      await page
        .getByLabel(labels.receivedExternal, { exact: true })
        .fill("SUPPLIER-0001");
      await page
        .getByLabel(`${labels.receivedLine} 1`, { exact: true })
        .fill("Hosting");
      await page
        .getByLabel(`${labels.receivedNet} 1`, { exact: true })
        .fill("100.01");
      await page
        .getByLabel(`${labels.draftVat} 1`, { exact: true })
        .fill("21.02");
      await page
        .getByLabel(labels.receivedRounding, { exact: true })
        .fill("-0.03");
      await page
        .getByRole("button", {
          name: new RegExp(`${labels.stockReceiveItem} 1`),
        })
        .click();
      await page
        .getByRole("option", { name: "Physical goods", exact: true })
        .click();
      await page
        .getByLabel(`${labels.stockReceiveQuantity} 1 (ks)`, { exact: true })
        .fill("2.5");
      await page
        .getByLabel(`${labels.stockAcquisitionCost} 1`, { exact: true })
        .fill("10.1234");
      await page
        .getByLabel(labels.stockIncidentalCost, { exact: true })
        .fill("20");
      await page
        .getByRole("button", { name: labels.receivedSave, exact: true })
        .click();
      await expect(page).toHaveURL(/received-invoices\/\d+$/);
      await expect(page.getByRole("heading", { level: 1 })).toContainText("PF");
      expect(
        (await (await page.request.get(`${stockBase}/stock/cards`)).json())[0]
          .quantity,
      ).toBe("2.5");
      const valuation = await (
        await page.request.get(`${stockBase}/stock/report?valuation=automatic`)
      ).json();
      expect(valuation.known_value).toBe("45.31");
      expect(valuation.items[0].unit_cost).toBe("18.1234");
      const detailUrl = page.url();
      await page.goto(`http://127.0.0.1:5173/#/companies/${alpha.id}/stock`);
      await page
        .getByRole("button", { name: new RegExp(labels.stockValuationMethod) })
        .click();
      await page
        .getByRole("option", { name: labels.stockAutomaticMethod, exact: true })
        .click();
      const overview = page.getByRole("region", {
        name: labels.stockOverviewTable,
        exact: true,
      });
      await expect(
        overview.getByText(english ? "18.1234 CZK" : "18,1234 CZK", {
          exact: true,
        }),
      ).toBeVisible();
      await page
        .getByRole("button", {
          name: labels.stockValuationBreakdown,
          exact: true,
        })
        .click();
      await expect(
        page.getByText(labels.stockCorrectionStock, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByText(labels.stockCorrectionIssued, { exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/stock-acquisition-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page.goto(detailUrl);
      await expect(page.getByRole("heading", { level: 1 })).toContainText("PF");
      await page.locator('input[type="file"]').setInputFiles({
        name: "source.txt",
        mimeType: "text/plain",
        buffer: Buffer.from("Synthetic original document"),
      });
      await expect(page.getByText("source.txt", { exact: true })).toBeVisible();
      await page
        .getByRole("button", { name: labels.receivedEdit, exact: true })
        .click();
      await page
        .getByLabel(labels.receivedDescription, { exact: true })
        .fill("Updated source record");
      await page
        .getByRole("button", { name: labels.receivedSave, exact: true })
        .click();
      await expect(
        page.getByRole("button", { name: labels.receivedEdit, exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: labels.paymentAdd, exact: true })
        .click();
      const dialog = page.getByRole("dialog");
      await expect(
        dialog.getByLabel(labels.paymentAmount, { exact: true }),
      ).toHaveValue("121.00");
      await dialog
        .getByLabel(labels.paymentAmount, { exact: true })
        .fill("21.00");
      await dialog
        .getByRole("button", { name: labels.paymentAdd, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(labels.paymentPartial, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("button", { name: labels.receivedEdit, exact: true }),
      ).toHaveCount(0);
      await page.screenshot({
        path: `docs/design/received-detail-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
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
      await page
        .getByRole("button", { name: labels.paymentReverse, exact: true })
        .click();
      await dialog
        .getByLabel(labels.paymentReason, { exact: true })
        .fill("Synthetic correction");
      await dialog
        .getByRole("button", { name: labels.paymentReverse, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(labels.paymentReversed, { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: labels.receivedCancel, exact: true })
        .click();
      await dialog
        .getByLabel(labels.paymentReason, { exact: true })
        .fill("Synthetic duplicate");
      await dialog
        .getByRole("button", { name: labels.receivedCancel, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(labels.receivedCancelled, { exact: true }),
      ).toBeVisible();
      await page.goto(`/#/companies/${alpha.id}/inbox`);
      await page.getByLabel(labels.inboxUpload, { exact: true }).setInputFiles({
        name: "synthetic.isdoc",
        mimeType: "application/xml",
        buffer: readFileSync("tests/fixtures/inbox/synthetic-basic.isdoc"),
      });
      const inboxRow = page
        .getByRole("row")
        .filter({ hasText: "synthetic.isdoc" });
      await expect(inboxRow).toBeVisible();
      await inboxRow
        .getByRole("button", { name: labels.inboxHistory, exact: true })
        .click();
      await expect(page.getByRole("dialog")).toContainText(labels.inboxUpload);
      await page.keyboard.press("Escape");
      await page.screenshot({
        path: `docs/design/inbox-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
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
      await inboxRow
        .getByRole("link", { name: labels.inboxCreate, exact: true })
        .click();
      await page
        .getByRole("button", { name: labels.inboxExtract, exact: true })
        .click();
      await expect(
        page.getByLabel(labels.receivedExternal, { exact: true }),
      ).toHaveValue("SYNTHETIC-001");
      await expect(
        page.getByLabel(`${labels.draftVat} 1`, { exact: true }),
      ).toHaveValue("21.02");
      await page
        .getByRole("button", { name: labels.receivedSave, exact: true })
        .click();
      await expect(page).toHaveURL(/received-invoices\/\d+$/);
      expect(messages).toHaveLength(0);
    } catch (error) {
      await page.screenshot({
        path: `test-results/received-failure-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      throw error;
    } finally {
      await page.unrouteAll({ behavior: "wait" }).catch(() => {});
      if (server && server.exitCode === null) {
        const exited = new Promise((resolve) => server!.once("exit", resolve));
        server.kill("SIGINT");
        await exited;
      }
      await new Promise<void>((resolve) => smtp.close(() => resolve()));
      sql(`DROP DATABASE ${database} WITH (FORCE)`);
      rmSync(directory, { recursive: true, force: true });
    }
  });
