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
  test(`purchase receipts and expense reports (${english ? "en" : "cs"})`, async ({
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
      await page.goto(`/#/companies/${alpha.id}/receipts`);
      await page
        .getByRole("link", { name: labels.receiptsNew, exact: true })
        .click();
      await page
        .getByLabel(`${labels.receivedLine} 1`, { exact: true })
        .fill("Office supplies");
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
        .getByRole("button", { name: labels.receivedSave, exact: true })
        .click();
      await expect(page).toHaveURL(/receipts\/\d+$/);
      const receiptId = page.url().split("/").at(-1)!;
      await expect(page.getByRole("heading", { level: 1 })).toContainText("UC");
      await expect(
        page.getByText(labels.paymentPaid, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("button", { name: labels.paymentAdd, exact: true }),
      ).toHaveCount(0);
      await page.locator('input[type="file"]').setInputFiles({
        name: "receipt.txt",
        mimeType: "text/plain",
        buffer: Buffer.from("Synthetic receipt original"),
      });
      await expect(
        page.getByText("receipt.txt", { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: labels.receivedEdit, exact: true })
        .click();
      await page
        .getByLabel(`${labels.receivedNet} 1`, { exact: true })
        .fill("200.01");
      await page.getByRole("button", { name: labels.paymentMethod }).click();
      await page
        .getByRole("option", { name: labels.receivedCash, exact: true })
        .click();
      await page
        .getByRole("button", { name: labels.receivedSave, exact: true })
        .click();
      await expect(
        page.getByRole("button", { name: labels.receivedEdit, exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("cell", {
          name: new RegExp(labels.receiptUpdatedPayment),
        }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/receipt-detail-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      const api = `http://127.0.0.1:${port}/api/companies/${alpha.id}/received-invoices`;
      const saved = await (
        await page.request.get(`${api}/${receiptId}`)
      ).json();
      const invoiceData = {
        ...saved.data,
        kind: "invoice",
        receipt_method: null,
        external_number: "SUPPLIER-TEST",
        supplier: { ...saved.data.supplier, name: "Vendor invoice" },
        lines: [
          { description: "Service", net: "100.01", vat: "21.02", rate: "21" },
        ],
      };
      const headers = {
        Origin: "http://127.0.0.1:5173",
        "X-Faktura-Request": "1",
      };
      const invoiceResponse = await page.request.post(api, {
        headers,
        data: { request_id: crypto.randomUUID(), data: invoiceData },
      });
      expect(invoiceResponse.ok()).toBe(true);
      const invoice = await invoiceResponse.json();
      const paid = await page.request.post(`${api}/${invoice.id}`, {
        headers,
        data: {
          request_id: crypto.randomUUID(),
          revision: invoice.revision,
          action: "pay",
          payment: {
            amount: "21.00",
            method: "bank_transfer",
            paid_on: saved.data.issued_on,
            note: "Partial invoice expense",
          },
        },
      });
      expect(paid.ok()).toBe(true);
      await page.goto(`/#/companies/${alpha.id}/expense-payments`);
      await expect(page.getByRole("table").getByRole("link")).toHaveCount(2);
      await expect(page.getByRole("table")).toContainText("Office supplies");
      await page.getByRole("button", { name: labels.paymentMethod }).click();
      await page
        .getByRole("option", { name: labels.receivedCash, exact: true })
        .click();
      await page
        .getByRole("button", { name: labels.expenseApply, exact: true })
        .click();
      await expect(page.getByRole("table").getByRole("link")).toHaveCount(1);
      await expect(page.getByRole("table")).toContainText("UC");
      await page.screenshot({
        path: `docs/design/expense-payments-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
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
      await page.goto(`/#/companies/${alpha.id}/expense-overview`);
      const metric = (label: string) =>
        page.getByRole("heading", { name: label, exact: true }).locator("..");
      await expect(metric(labels.expensePurchases)).toContainText(/342[,.]00/);
      await expect(metric(labels.expensePaid)).toContainText(/242[,.]00/);
      await expect(metric(labels.expenseOutstanding)).toContainText(
        /100[,.]00/,
      );
      await page.screenshot({
        path: `docs/design/expense-overview-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
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
      await page.goto(`/#/companies/${_beta.id}/expense-overview`);
      await expect(metric(labels.expensePurchases)).toContainText(/0[,.]00/);
      await page.goto(`/#/companies/${alpha.id}/receipts/${receiptId}`);
      await page
        .getByRole("button", { name: labels.receivedCancel, exact: true })
        .click();
      const dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.paymentReason, { exact: true })
        .fill("Duplicate receipt");
      await dialog
        .getByRole("button", { name: labels.receivedCancel, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(labels.receivedCancelled, { exact: true }),
      ).toBeVisible();
      await page.goto(`/#/companies/${alpha.id}/expense-overview`);
      await expect(metric(labels.expensePurchases)).toContainText(/121[,.]00/);
      await expect(metric(labels.expensePaid)).toContainText(/21[,.]00/);
      expect(messages).toHaveLength(0);
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
