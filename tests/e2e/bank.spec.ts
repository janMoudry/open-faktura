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
  test(`bank accounts, native XML and suggested matching (${english ? "en" : "cs"})`, async ({
    page,
  }, info) => {
    test.setTimeout(180000);
    const database = `company_e2e_${randomBytes(8).toString("hex")}`;
    const directory = mkdtempSync(join(tmpdir(), "faktura-auth-"));
    const secret = randomBytes(32).toString("hex");
    writeFileSync(join(directory, "bootstrap"), secret, { mode: 0o600 });
    writeFileSync(
      join(directory, "bank-key"),
      randomBytes(32).toString("hex"),
      { mode: 0o600 },
    );
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
          BANK_ENCRYPTION_KEY_FILE: join(directory, "bank-key"),
          BANK_GATEWAYS_JSON: JSON.stringify([
            {
              id: "synthetic",
              label: "Synthetic gateway",
              url: "https://127.0.0.1:9/transactions",
              format: "camt053",
            },
          ]),
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
      const alpha = await makeProfile("Alpha"),
        _beta = await makeProfile("Beta");
      const labels = JSON.parse(
        readFileSync(
          `apps/web/src/locales/${english ? "en" : "cs"}.json`,
          "utf8",
        ),
      );
      const base = `http://127.0.0.1:${port}/api/companies/${alpha.id}`;
      const post = async (path: string, data: unknown) => {
        const r = await page.request.post(`${base}${path}`, {
          headers: {
            Origin: "http://127.0.0.1:5173",
            "X-Faktura-Request": "1",
          },
          data,
        });
        expect(r.ok(), await r.text()).toBe(true);
        return r.json();
      };
      const fixture = JSON.parse(
        readFileSync("tests/fixtures/invoice-draft.json", "utf8"),
      );
      fixture.issued_on = "2026-10-01";
      fixture.due_on = "2026-10-15";
      fixture.tax_on = null;
      fixture.payment.method = "cash";
      fixture.payment.variable_symbol = "000123";
      fixture.supplier.registration_number = "12345678";
      fixture.calculation.tax_mode = "non_payer";
      fixture.calculation.lines[0].vat_percent = "0";
      fixture.calculation.lines[0].unit_price = "100";
      const draft = await post("/invoices", {
        request_id: crypto.randomUUID(),
        data: fixture,
      });
      const issued = await post(`/invoices/${draft.id}/issue`, {
        request_id: crypto.randomUUID(),
        revision: draft.revision,
      });
      await page.goto(`/#/companies/${alpha.id}/bank`);
      await page
        .getByRole("button", { name: labels.bankAddAccount, exact: true })
        .click();
      let dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.bankName, { exact: true })
        .fill("Operating CZK");
      await dialog
        .getByLabel(labels.bankIban, { exact: true })
        .fill("CZ6508000000192000145399");
      await dialog
        .getByRole("button", { name: labels.bankSave, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await page
        .getByRole("button", { name: labels.connectionTitle, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await expect(
        dialog.getByRole("button", {
          name: labels.connectionTest,
          exact: true,
        }),
      ).toBeDisabled();
      await dialog
        .getByRole("button", {
          name: `Fio API Banking ${labels.connectionProvider}`,
          exact: true,
        })
        .click();
      await page
        .getByRole("option", { name: "Synthetic gateway", exact: true })
        .click();
      await dialog
        .getByLabel(labels.connectionToken, { exact: true })
        .fill("synthetic-token-not-a-real-secret");
      await dialog
        .getByRole("button", { name: labels.bankSave, exact: true })
        .click();
      await expect(
        dialog.getByLabel(labels.connectionToken, { exact: true }),
      ).toHaveValue("");
      await expect(
        dialog.getByText(labels.connectionConfigured, { exact: true }),
      ).toBeVisible();
      const tested = page.waitForResponse(
        (r) =>
          r.url().endsWith("/bank/connection") &&
          r.request().method() === "POST" &&
          r.request().postDataJSON()?.action === "test",
      );
      await dialog
        .getByRole("button", { name: labels.connectionTest, exact: true })
        .click();
      const testResponse = await tested;
      expect(testResponse.status()).toBe(200);
      expect((await testResponse.json()).last_run.kind).toBe("test");
      await expect(
        dialog.getByText(labels.connectionNetwork, { exact: true }),
      ).toBeVisible();
      await dialog
        .getByRole("button", {
          name: `${labels.connectionAutoOff} ${labels.connectionAutoInterval}`,
          exact: true,
        })
        .click();
      await page
        .getByRole("option", { name: labels.connectionEvery15, exact: true })
        .click();
      await dialog
        .getByRole("button", { name: labels.connectionAutoSave, exact: true })
        .click();
      await expect(
        dialog.getByText(labels.connectionAutoOn, { exact: true }),
      ).toBeVisible();
      sql(
        "UPDATE bank_connection SET next_run_at=now()-interval '1 minute' WHERE schedule_enabled",
        database,
      );
      await expect
        .poll(
          () =>
            sql(
              "SELECT count(*) FROM bank_connection_run WHERE kind='sync' AND status='failed'",
              database,
            ).toString(),
          { timeout: 20000 },
        )
        .toMatch(/\n\s*1\s*\n/);
      await dialog
        .getByRole("button", { name: labels.connectionRefresh, exact: true })
        .click();
      await expect(
        dialog.getByText(labels.connectionNetwork, { exact: true }),
      ).toBeVisible();
      await dialog.getByText(labels.matchingEnable, { exact: true }).click();
      await dialog
        .getByRole("button", { name: labels.matchingSave, exact: true })
        .click();
      await expect(
        dialog.getByText(labels.matchingOn, { exact: true }),
      ).toBeVisible();
      sql(
        "UPDATE bank_connection SET matching_next_at=now()-interval '1 minute' WHERE matching_enabled",
        database,
      );
      await expect
        .poll(
          () =>
            sql("SELECT count(*) FROM bank_matching_run", database).toString(),
          { timeout: 20000 },
        )
        .toMatch(/\n\s*1\s*\n/);
      await dialog
        .getByRole("button", { name: labels.connectionRefresh, exact: true })
        .click();
      await expect(
        dialog.getByText(
          labels.matchingCounts
            .replace("{{checked}}", "0")
            .replace("{{matched}}", "0"),
          { exact: false },
        ),
      ).toBeVisible();
      await dialog.getByText(labels.matchingEnable, { exact: true }).click();
      await dialog
        .getByRole("button", { name: labels.matchingSave, exact: true })
        .click();
      await expect(
        dialog.getByText(labels.matchingOff, { exact: true }),
      ).toBeVisible();
      await dialog
        .getByRole("heading", { name: labels.connectionTitle })
        .scrollIntoViewIfNeeded();
      await page.screenshot({
        path: `docs/design/bank-connection-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      await dialog
        .getByRole("button", { name: labels.connectionDisconnect, exact: true })
        .click();
      await dialog
        .getByRole("button", {
          name: labels.connectionDisconnectConfirm,
          exact: true,
        })
        .click();
      await expect(
        dialog.getByText(labels.connectionDisconnected, { exact: true }),
      ).toBeVisible();
      await dialog
        .getByRole("button", { name: labels.bankClose, exact: true })
        .click();
      const balancePanel = page.getByRole("region", {
        name: labels.bankBalance,
        exact: true,
      });
      await expect(
        balancePanel.getByText(labels.bankOpeningMissing, { exact: true }),
      ).toBeVisible();
      await balancePanel
        .getByRole("button", { name: labels.bankOpeningEdit, exact: true })
        .click();
      const spins = balancePanel.getByRole("spinbutton");
      await spins.nth(6).fill("1");
      await spins.nth(7).fill("1");
      await spins.nth(8).fill("2026");
      await balancePanel
        .getByLabel(labels.cashOpening, { exact: true })
        .fill("1000.00");
      await balancePanel
        .getByLabel(labels.bankReason, { exact: true })
        .fill("Synthetic opening balance");
      await balancePanel
        .getByRole("button", { name: labels.bankSave, exact: true })
        .click();
      await expect(
        balancePanel.getByText(labels.bankOpeningMissing, { exact: true }),
      ).toHaveCount(0);
      await expect(
        balancePanel.getByRole("button", {
          name: labels.bankOpeningEdit,
          exact: true,
        }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: labels.bankImport, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      const csv = readFileSync("tests/fixtures/bank/camt053.xml", "utf8")
        .replace("100.01", "100.00")
        .replace("Test payment", "Invoice payment");
      await dialog
        .getByRole("button", {
          name: `${labels.bankFormatCsv} ${labels.bankFormat}`,
          exact: true,
        })
        .click();
      await page
        .getByRole("option", { name: labels.bankFormatCamt, exact: true })
        .click();
      await dialog.getByLabel(labels.bankFile).setInputFiles({
        name: "synthetic.xml",
        mimeType: "application/xml",
        buffer: Buffer.from(csv),
      });
      await dialog
        .getByRole("button", { name: labels.bankPreview, exact: true })
        .click();
      await expect(dialog.getByRole("status")).toContainText("2");
      await page.screenshot({
        path: `docs/design/bank-import-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      await dialog
        .getByRole("button", { name: labels.bankConfirmImport, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(page.getByRole("link", { name: /^B2026/ })).toHaveCount(2);
      await page.screenshot({
        path: `docs/design/bank-register-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      await page
        .getByRole("button", { name: labels.bankImport, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", {
          name: `${labels.bankFormatCsv} ${labels.bankFormat}`,
          exact: true,
        })
        .click();
      await page
        .getByRole("option", { name: labels.bankFormatCamt, exact: true })
        .click();
      await dialog.getByLabel(labels.bankFile).setInputFiles({
        name: "synthetic.xml",
        mimeType: "application/xml",
        buffer: Buffer.from(csv),
      });
      await dialog
        .getByRole("button", { name: labels.bankPreview, exact: true })
        .click();
      await expect(
        dialog.getByText(labels.bankDuplicate, { exact: true }),
      ).toHaveCount(2);
      await expect(
        dialog.getByRole("button", {
          name: labels.bankConfirmImport,
          exact: true,
        }),
      ).toBeDisabled();
      await dialog
        .getByRole("button", { name: labels.bankClose, exact: true })
        .click();
      await page
        .getByRole("row")
        .filter({ hasText: "Invoice payment" })
        .getByRole("link")
        .click();
      await expect(
        page.getByRole("heading", { name: labels.bankMatchTitle, exact: true }),
      ).toBeVisible();
      await expect(
        page.getByText(labels.bankSuggestionsUnique, { exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/bank-suggestions-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      await page
        .getByRole("button", { name: labels.bankChooseSuggestion, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await expect(dialog).toContainText(labels.bankMatchNewHelp);
      await dialog
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByRole("button", { name: labels.bankUnmatch, exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/bank-detail-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      let payments = await (
        await page.request.get(`${base}/invoices/${draft.id}/payments`)
      ).json();
      expect(payments.summary.paid).toBe("100.00");
      await page
        .getByRole("button", { name: labels.bankUnmatch, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.bankReason, { exact: true })
        .fill("Wrong invoice");
      await dialog
        .getByRole("button", { name: labels.bankUnmatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page
          .getByRole("heading", { name: labels.bankHistory, exact: true })
          .locator("..")
          .getByText(labels.bankDetached, { exact: false }),
      ).toBeVisible();
      payments = await (
        await page.request.get(`${base}/invoices/${draft.id}/payments`)
      ).json();
      expect(payments.summary.paid).toBe("0.00");
      await page
        .getByRole("link", { name: labels.bankBack, exact: true })
        .click();
      // Batch matching uses only displayed transactions, with explicit review and stale recovery.
      const batchFixture = structuredClone(fixture);
      batchFixture.payment.variable_symbol = "999987";
      const batchDraft = await post("/invoices", {
        request_id: crypto.randomUUID(),
        data: batchFixture,
      });
      await post(`/invoices/${batchDraft.id}/issue`, {
        request_id: crypto.randomUUID(),
        revision: batchDraft.revision,
      });
      const bankAccount = (
        await (await page.request.get(`${base}/bank/accounts`)).json()
      )[0];
      await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "manual",
        account_id: bankAccount.id,
        data: {
          external_id: "",
          booked_on: "2026-10-01",
          amount: "300.00",
          currency: "CZK",
          variable_symbol: "999987",
          counterparty: "Batch customer",
          description: "Batch exact payment",
        },
      });
      await page.reload();
      await expect(page.getByRole("link", { name: /^B2026/ })).toHaveCount(3);
      await page
        .getByRole("button", { name: labels.bankBatchOpen, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      const confirmBatch = dialog.getByRole("button", {
        name: labels.bankBatchConfirm.replace("{{count}}", "1"),
        exact: true,
      });
      await expect(confirmBatch).toBeEnabled();
      await expect(
        dialog.getByText(labels.bankBatchPartial, { exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/bank-batch-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      const archived = await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "account",
        id: bankAccount.id,
        revision: bankAccount.revision,
        data: bankAccount.data,
        archived: true,
      });
      await confirmBatch.click();
      await expect(
        dialog.getByText(labels.bankBatchStale, { exact: true }),
      ).toBeVisible();
      await expect(confirmBatch).toBeDisabled();
      expect(
        (
          await (
            await page.request.get(`${base}/invoices/${batchDraft.id}/payments`)
          ).json()
        ).summary.paid,
      ).toBe("0.00");
      await dialog
        .getByRole("button", { name: labels.bankBatchRefresh, exact: true })
        .click();
      await expect(
        dialog.getByText(labels.bankBatchEmpty, { exact: true }),
      ).toBeVisible();
      await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "account",
        id: bankAccount.id,
        revision: archived.value.revision,
        data: bankAccount.data,
        archived: false,
      });
      await dialog
        .getByRole("button", { name: labels.bankBatchRefresh, exact: true })
        .click();
      await expect(confirmBatch).toBeEnabled();
      await confirmBatch.click();
      await expect(dialog).not.toBeVisible();
      expect(
        (
          await (
            await page.request.get(`${base}/invoices/${batchDraft.id}/payments`)
          ).json()
        ).summary.paid,
      ).toBe("300.00");
      expect(
        (
          await (
            await page.request.get(`${base}/invoices/${draft.id}/payments`)
          ).json()
        ).summary.paid,
      ).toBe("0.00");
      await page
        .getByRole("button", { name: labels.bankManual, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog.getByLabel(labels.bankAmount, { exact: true }).fill("13.25");
      await dialog
        .getByLabel(labels.bankDescription, { exact: true })
        .fill("Manual test movement");
      await dialog
        .getByRole("button", { name: labels.bankSave, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(page.getByRole("link", { name: /^B2026/ })).toHaveCount(4);
      const splitDraft = await post("/invoices", {
        request_id: crypto.randomUUID(),
        data: fixture,
      });
      const splitIssued = await post(`/invoices/${splitDraft.id}/issue`, {
        request_id: crypto.randomUUID(),
        revision: splitDraft.revision,
      });
      const splitMovement = await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "manual",
        account_id: bankAccount.id,
        data: {
          external_id: "",
          booked_on: "2026-10-14",
          amount: "650.01",
          currency: "CZK",
          variable_symbol: "",
          counterparty: "Synthetic split",
          description: "Split fixture",
        },
      });
      await page.goto(
        `/#/companies/${alpha.id}/bank/${splitMovement.value.id}`,
      );
      await page
        .getByRole("button", { name: labels.bankSplit, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      for (const number of [issued.number, splitIssued.number]) {
        await dialog
          .getByLabel(labels.bankCandidateSearch, { exact: true })
          .fill(number);
        await dialog
          .getByRole("button", { name: labels.bankSearchAction, exact: true })
          .click();
        const add = dialog.getByRole("button", {
          name: labels.bankSplitAdd,
          exact: true,
        });
        await expect(add).toHaveCount(1);
        await add.click();
      }
      await dialog
        .getByLabel(
          labels.bankSplitAmount.replace("{{number}}", splitIssued.number),
          { exact: true },
        )
        .fill("250.02");
      await expect(dialog.getByText(/99[.,]99/)).toBeVisible();
      await page.screenshot({
        path: `docs/design/bank-split-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      await dialog
        .getByRole("button", { name: labels.bankSplitSave, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(labels.bankPartial, { exact: true }),
      ).toBeVisible();
      const savedSplit = await (
        await page.request.get(
          `${base}/bank/movements/${splitMovement.value.id}`,
        )
      ).json();
      expect(savedSplit.remaining).toBe("99.99");
      expect(savedSplit.matches).toHaveLength(2);
      await page
        .getByRole("button", { name: labels.bankUnmatch, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.bankReason, { exact: true })
        .fill("Synthetic split reversal");
      await dialog
        .getByRole("button", { name: labels.bankUnmatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      expect(
        (
          await (
            await page.request.get(`${base}/invoices/${splitDraft.id}/payments`)
          ).json()
        ).summary.paid,
      ).toBe("0.00");
      // Advance payments use their own ledger; UI links must retain the document type.
      const advance = await post("/sales", {
        request_id: crypto.randomUUID(),
        kind: "advance",
        data: {
          name: "Synthetic treasury advance",
          maturity_days: 14,
          status: "created",
          invoice: structuredClone(fixture),
        },
      });
      const advanceMovement = await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "manual",
        account_id: bankAccount.id,
        data: {
          external_id: "",
          booked_on: advance.data.invoice.issued_on,
          amount: "25",
          currency: "CZK",
          variable_symbol: "",
          counterparty: "Synthetic advance",
          description: "Advance payment",
        },
      });
      await page.goto(
        `/#/companies/${alpha.id}/bank/${advanceMovement.value.id}`,
      );
      await page
        .getByRole("row")
        .filter({
          has: page.getByRole("link", { name: advance.number, exact: true }),
        })
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByRole("link", { name: advance.number, exact: true }).first(),
      ).toHaveAttribute(
        "href",
        `#/companies/${alpha.id}/advances/${advance.id}`,
      );
      const fxData = structuredClone(fixture);
      fxData.calculation.currency = "EUR";
      fxData.calculation.exchange_rate = { rate: "25", units: "1" };
      fxData.calculation.lines = fxData.calculation.lines.slice(0, 1);
      Object.assign(fxData.calculation.lines[0], {
        quantity: "1",
        unit_price: "100",
        vat_percent: "0",
        discount_percent: "0",
      });
      fxData.calculation.discount_percent = "0";
      fxData.calculation.rounding = "none";
      fxData.calculation.tax_mode = "non_payer";
      fxData.payment.method = "bank_transfer";
      fxData.payment.bank_account = "12345678/0100";
      const fxDraft = await post("/invoices", {
        request_id: crypto.randomUUID(),
        data: fxData,
      });
      await post(`/invoices/${fxDraft.id}/issue`, {
        request_id: crypto.randomUUID(),
        revision: fxDraft.revision,
      });
      const fxMovement = await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "manual",
        account_id: bankAccount.id,
        data: {
          external_id: "",
          booked_on: fxData.issued_on,
          amount: "2490",
          currency: "CZK",
          variable_symbol: "",
          counterparty: "Synthetic FX",
          description: "Synthetic currency matching",
        },
      });
      await page.goto(`/#/companies/${alpha.id}/bank/${fxMovement.value.id}`);
      await page
        .getByRole("button", { name: labels.bankForeignTitle, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", {
          name: new RegExp(labels.bankForeignDocument),
        })
        .click();
      await page.getByRole("option").filter({ hasText: "EUR" }).first().click();
      await expect(
        dialog.getByLabel(
          labels.bankForeignDocumentAmount.replace("{{currency}}", "EUR"),
          { exact: true },
        ),
      ).toHaveValue("100.00");
      await page.screenshot({
        path: `docs/design/bank-foreign-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await dialog
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(new RegExp(english ? "difference" : "rozdíl")).first(),
      ).toContainText(english ? "-10.00" : "-10,00");
      const fxPaid = await (
        await page.request.get(`${base}/invoices/${fxDraft.id}/payments`)
      ).json();
      expect(fxPaid.summary.outstanding).toBe("0.00");
      await page.reload();
      await expect(
        page.getByText(new RegExp(english ? "difference" : "rozdíl")).first(),
      ).toContainText(english ? "-10.00" : "-10,00");
      const recordedDraft = await post("/invoices", {
        request_id: crypto.randomUUID(),
        data: fxData,
      });
      await post(`/invoices/${recordedDraft.id}/issue`, {
        request_id: crypto.randomUUID(),
        revision: recordedDraft.revision,
      });
      const recorded = await post(`/invoices/${recordedDraft.id}/payments`, {
        request_id: crypto.randomUUID(),
        revision: 0,
        payment: {
          amount: "100",
          paid_on: fxData.issued_on,
          method: "bank_transfer",
          note: "Existing synthetic FX payment",
        },
      });
      const recordedMovement = await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "manual",
        account_id: bankAccount.id,
        data: {
          external_id: "",
          booked_on: fxData.issued_on,
          amount: "2480",
          currency: "CZK",
          variable_symbol: "",
          counterparty: "Synthetic recorded FX",
          description: "Existing FX matching",
        },
      });
      await page.goto(
        `/#/companies/${alpha.id}/bank/${recordedMovement.value.id}`,
      );
      await page
        .getByRole("button", { name: labels.bankForeignTitle, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", { name: new RegExp(labels.bankForeignDocument) })
        .click();
      await page
        .getByRole("option")
        .filter({ hasText: recorded.items[0].id })
        .click();
      await expect(
        dialog.getByLabel(
          labels.bankForeignDocumentAmount.replace("{{currency}}", "EUR"),
          { exact: true },
        ),
      ).toBeDisabled();
      await expect(
        dialog.getByText(labels.bankForeignExistingHelp, { exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/bank-foreign-existing-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await dialog
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      let unchanged = await (
        await page.request.get(`${base}/invoices/${recordedDraft.id}/payments`)
      ).json();
      expect(unchanged.items).toEqual(recorded.items);
      expect(unchanged.summary.revision).toBe(recorded.summary.revision);
      await page
        .getByRole("button", { name: labels.bankUnmatch, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("textbox")
        .fill("Synthetic unlink existing payment");
      await dialog
        .getByRole("button", { name: labels.bankUnmatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      unchanged = await (
        await page.request.get(`${base}/invoices/${recordedDraft.id}/payments`)
      ).json();
      expect(unchanged.items).toEqual(recorded.items);
      expect(unchanged.summary.outstanding).toBe("0.00");
      await page.goto(`/#/companies/${alpha.id}/reports`);
      const fxRegion = page.getByRole("region", {
        name: labels.fxReportTitle,
        exact: true,
      });
      await expect(
        fxRegion
          .getByText(english ? "-10.00 CZK" : "-10,00 Kč", { exact: true })
          .first(),
      ).toBeVisible();
      await expect(
        fxRegion.getByRole("link", {
          name: fxMovement.value.number,
          exact: true,
        }),
      ).toHaveAttribute(
        "href",
        `#/companies/${alpha.id}/bank/${fxMovement.value.id}`,
      );
      await expect(
        fxRegion.getByRole("link", {
          name: recordedMovement.value.number,
          exact: true,
        }),
      ).toHaveCount(0);
      await fxRegion.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: `docs/design/exchange-difference-report-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await fxRegion
        .getByRole("link", { name: fxMovement.value.number, exact: true })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toContainText(
        fxMovement.value.number,
      );
      const advancePayment = await (
        await page.request.get(
          `${base}/bank/movements/${advanceMovement.value.id}`,
        )
      ).json();
      expect(advancePayment.matches[0].target_kind).toBe("advance");
      expect(advancePayment.allocated).toBe("25.00");
      await page.goto(`/#/companies/${alpha.id}/bank`);
      await page
        .getByRole("button", { name: labels.bankAccounts, exact: true })
        .click();
      await page
        .getByRole("button", { name: labels.bankEdit, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", { name: new RegExp(labels.bankAvailability) })
        .click();
      await page
        .getByRole("option", { name: labels.bankArchived, exact: true })
        .click();
      await dialog
        .getByRole("button", { name: labels.bankSave, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByRole("button", { name: labels.bankImport, exact: true }),
      ).toBeDisabled();
      // Native EUR amounts remain EUR through account creation, import and detail.
      await page
        .getByRole("button", { name: labels.bankAddAccount, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.bankName, { exact: true })
        .fill("Operating EUR");
      await dialog
        .getByLabel(labels.bankIban, { exact: true })
        .fill("DE89370400440532013001");
      await dialog
        .getByRole("button", { name: labels.bankSave, exact: true })
        .click();
      await expect(dialog.getByText(labels.bankIbanInvalid)).toBeVisible();
      await dialog
        .getByLabel(labels.bankIban, { exact: true })
        .fill("de89 3704 0044 0532 0130 00");
      await dialog
        .getByRole("button", { name: new RegExp(labels.bankCurrency) })
        .click();
      await page.getByRole("option", { name: "EUR", exact: true }).click();
      await page.screenshot({
        path: `docs/design/bank-foreign-iban-${english ? "en" : "cs"}-${info.project.name}.png`,
        animations: "disabled",
      });
      await dialog
        .getByRole("button", { name: labels.bankSave, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await page
        .getByRole("button", { name: labels.bankImport, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog.getByLabel(labels.bankFile).setInputFiles({
        name: "eur.csv",
        mimeType: "text/csv",
        buffer: Buffer.from(
          "external_id,booked_on,amount,currency,variable_symbol,counterparty,description\neur1,2026-10-01,123.45,EUR,123,Synthetic EUR,EUR import\n",
        ),
      });
      await dialog
        .getByRole("button", { name: labels.bankPreview, exact: true })
        .click();
      await dialog
        .getByRole("button", { name: labels.bankConfirmImport, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      const eurRow = page.getByRole("row").filter({ hasText: "EUR import" });
      await expect(eurRow).toContainText(english ? "123.45 EUR" : "123,45 EUR");
      const nativeData = structuredClone(fxData);
      nativeData.calculation.lines[0].unit_price = "123.45";
      const nativeDraft = await post("/invoices", {
        request_id: crypto.randomUUID(),
        data: nativeData,
      });
      const nativeIssued = await post(`/invoices/${nativeDraft.id}/issue`, {
        request_id: crypto.randomUUID(),
        revision: nativeDraft.revision,
      });
      await eurRow.getByRole("link").click();
      await expect(
        page.getByText(labels.bankNativeMatchingHelp, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("button", {
          name: labels.bankForeignTitle,
          exact: true,
        }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/bank-currencies-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      const nativeCandidate = page
        .getByRole("row")
        .filter({ hasText: nativeIssued.number });
      await expect(nativeCandidate).toContainText(
        english ? "123.45 EUR" : "123,45 EUR",
      );
      await nativeCandidate
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await expect(dialog).toContainText(english ? "123.45 EUR" : "123,45 EUR");
      await dialog
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      const nativePaid = await (
        await page.request.get(`${base}/invoices/${nativeDraft.id}/payments`)
      ).json();
      expect(nativePaid.summary.outstanding).toBe("0.00");
      expect(nativePaid.items).toHaveLength(1);
      expect(nativePaid.items[0].amount).toBe("123.45");
      await page.screenshot({
        path: `docs/design/bank-native-matching-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("button", { name: labels.bankUnmatch, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog.getByRole("textbox").fill("Synthetic EUR reversal");
      await dialog
        .getByRole("button", { name: labels.bankUnmatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      expect(
        (
          await (
            await page.request.get(
              `${base}/invoices/${nativeDraft.id}/payments`,
            )
          ).json()
        ).summary.outstanding,
      ).toBe("123.45");
      const eurAccount = (
        await (await page.request.get(`${base}/bank/accounts`)).json()
      ).find((a: { data: { currency: string } }) => a.data.currency === "EUR");
      expect(eurAccount.data.iban).toBe("DE89370400440532013000");
      await page.goto(`/#/companies/${alpha.id}/bank?account=${eurAccount.id}`);
      await page
        .getByRole("button", { name: labels.bankBatchOpen, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await expect(dialog).toContainText(english ? "123.45 EUR" : "123,45 EUR");
      await dialog
        .getByRole("button", {
          name: labels.bankBatchConfirm.replace("{{count}}", "1"),
          exact: true,
        })
        .click();
      await expect(dialog).not.toBeVisible();
      expect(
        (
          await (
            await page.request.get(
              `${base}/invoices/${nativeDraft.id}/payments`,
            )
          ).json()
        ).summary.outstanding,
      ).toBe("0.00");
      const automaticData = structuredClone(fxData);
      automaticData.payment.variable_symbol = "991122";
      const automaticDraft = await post("/invoices", {
        request_id: crypto.randomUUID(),
        data: automaticData,
      });
      await post(`/invoices/${automaticDraft.id}/issue`, {
        request_id: crypto.randomUUID(),
        revision: automaticDraft.revision,
      });
      await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "manual",
        account_id: eurAccount.id,
        data: {
          external_id: "",
          booked_on: automaticData.issued_on,
          amount: "100",
          currency: "EUR",
          variable_symbol: "991122",
          counterparty: "Synthetic automatic EUR",
          description: "Automatic EUR",
        },
      });
      await page
        .getByRole("button", { name: labels.connectionTitle, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", { name: new RegExp(labels.bankAccount) })
        .click();
      await page
        .getByRole("option", { name: "Operating EUR", exact: true })
        .click();
      await expect(
        dialog.getByRole("switch", {
          name: labels.matchingEnable,
          exact: true,
        }),
      ).not.toBeChecked();
      await dialog.getByText(labels.matchingEnable, { exact: true }).click();
      await dialog
        .getByRole("button", { name: labels.matchingSave, exact: true })
        .click();
      await expect(
        dialog.getByText(labels.matchingOn, { exact: true }),
      ).toBeVisible();
      sql(
        `UPDATE bank_connection SET matching_next_at=now()-interval '1 minute' WHERE account_id=${eurAccount.id} AND matching_enabled`,
        database,
      );
      await expect
        .poll(
          async () =>
            (
              await (
                await page.request.get(
                  `${base}/invoices/${automaticDraft.id}/payments`,
                )
              ).json()
            ).summary.outstanding,
          { timeout: 20000 },
        )
        .toBe("0.00");
      await dialog
        .getByRole("button", { name: labels.connectionRefresh, exact: true })
        .click();
      await page.screenshot({
        path: `docs/design/bank-native-automation-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await dialog.getByText(labels.matchingEnable, { exact: true }).click();
      await dialog
        .getByRole("button", { name: labels.matchingSave, exact: true })
        .click();
      await expect(
        dialog.getByText(labels.matchingOff, { exact: true }),
      ).toBeVisible();
      await dialog
        .getByRole("button", { name: labels.bankClose, exact: true })
        .click();
      const eurAdvance = await post("/sales", {
        request_id: crypto.randomUUID(),
        kind: "advance",
        data: {
          name: "Synthetic EUR advance",
          maturity_days: 14,
          status: "created",
          invoice: structuredClone(fxData),
        },
      });
      const eurAdvanceMovement = await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "manual",
        account_id: eurAccount.id,
        data: {
          external_id: "",
          booked_on: eurAdvance.data.invoice.issued_on,
          amount: "25",
          currency: "EUR",
          variable_symbol: "",
          counterparty: "Synthetic EUR advance",
          description: "EUR advance payment",
        },
      });
      await page.goto(
        `/#/companies/${alpha.id}/bank/${eurAdvanceMovement.value.id}`,
      );
      await expect(
        page.getByText(labels.bankNativeMatchingHelp, { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("row")
        .filter({
          has: page.getByRole("link", { name: eurAdvance.number, exact: true }),
        })
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page
          .getByRole("link", { name: eurAdvance.number, exact: true })
          .first(),
      ).toHaveAttribute(
        "href",
        `#/companies/${alpha.id}/advances/${eurAdvance.id}`,
      );
      await page.screenshot({
        path: `docs/design/bank-native-treasury-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      // EUR bank allocation to an USD invoice: keep both original currencies.
      const crossData = structuredClone(fxData);
      crossData.calculation.currency = "USD";
      crossData.calculation.exchange_rate = { rate: "22", units: "1" };
      const crossDraft = await post("/invoices", {
        request_id: crypto.randomUUID(),
        data: crossData,
      });
      const crossIssued = await post(`/invoices/${crossDraft.id}/issue`, {
        request_id: crypto.randomUUID(),
        revision: crossDraft.revision,
      });
      const crossMovement = await post("/bank/commands", {
        request_id: crypto.randomUUID(),
        action: "manual",
        account_id: eurAccount.id,
        data: {
          external_id: "",
          booked_on: crossData.issued_on,
          amount: "90",
          currency: "EUR",
          variable_symbol: "",
          counterparty: "Synthetic USD invoice",
          description: "Cross currency",
        },
      });
      await page.goto(
        `/#/companies/${alpha.id}/bank/${crossMovement.value.id}`,
      );
      await page
        .getByRole("button", { name: labels.bankForeignTitle, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", { name: new RegExp(labels.bankForeignDocument) })
        .click();
      await page
        .getByRole("option")
        .filter({ hasText: crossIssued.number })
        .click();
      await expect(
        dialog.getByLabel(
          labels.bankForeignBankAmount.replace("{{currency}}", "EUR"),
          { exact: true },
        ),
      ).toHaveValue("90.00");
      await dialog
        .getByLabel(labels.bankForeignRate.replace("{{currency}}", "EUR"), {
          exact: true,
        })
        .fill("25");
      await dialog
        .getByRole("button", { name: labels.bankMatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(new RegExp(english ? "difference" : "rozdíl")).first(),
      ).toContainText(english ? "50.00" : "50,00");
      await page.reload();
      await expect(
        page.getByText(new RegExp(english ? "difference" : "rozdíl")).first(),
      ).toContainText(english ? "50.00" : "50,00");
      const crossPayment = await (
        await page.request.get(`${base}/invoices/${crossDraft.id}/payments`)
      ).json();
      expect(crossPayment.summary.outstanding).toBe("0.00");
      await page.screenshot({
        path: `docs/design/bank-cross-currency-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page.goto(`/#/companies/${_beta.id}/bank`);
      await expect(
        page.getByText(labels.bankNoAccounts, { exact: true }),
      ).toBeVisible();
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
