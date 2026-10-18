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
  test(`cash registers, balances and cash payment matching (${english ? "en" : "cs"})`, async ({
    page,
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
      if (english) {
        await page.getByRole("button", { name: "Uživatelské menu" }).click();
        await page.getByRole("button", { name: /Jazyk aplikace/ }).click();
        await page.getByRole("option", { name: /English/ }).click();
        await page.keyboard.press("Escape");
      }
      const settings = {
        name: "Alpha",
        entity_type: "individual",
        registration_number: "12345678",
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
      // Sales receipts are separate from cash vouchers; no customer is required.
      await page.goto(`/#/companies/${alpha.id}/retail/new`);
      await page
        .getByLabel(labels.draftDescription, { exact: true })
        .fill("Synthetic retail service");
      await page.getByLabel(labels.draftUnitPrice, { exact: true }).fill("100");
      await page
        .getByRole("button", { name: labels.retailAllCash, exact: true })
        .click();
      await page
        .getByRole("button", { name: labels.retailSave, exact: true })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "PR20260001",
      );
      const retailId = page.url().split("/").at(-1)!;
      const retailPdf = await page.request.get(
        `${base}/retail/${retailId}/pdf`,
      );
      expect(retailPdf.ok()).toBe(true);
      expect((await retailPdf.body()).subarray(0, 4).toString()).toBe("%PDF");
      await page.screenshot({
        path: `docs/design/retail-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      await page.goto(`/#/companies/${alpha.id}/cash`);
      await page
        .getByRole("button", { name: labels.cashAddAccount, exact: true })
        .click();
      let dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.cashName, { exact: true })
        .fill("Office cash");
      await dialog
        .getByLabel(labels.cashOpening, { exact: true })
        .fill("1000.00");
      await dialog
        .getByRole("button", { name: labels.cashSave, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByRole("region", { name: labels.cashBalance }),
      ).toContainText(/1[\s\u00a0,]?000/);
      await page.goto(`/#/companies/${alpha.id}/retail`);
      await page
        .getByRole("checkbox", { name: labels.retailSelect + " PR20260001" })
        .check();
      await page
        .getByRole("button", { name: new RegExp(labels.retailPostingAccount) })
        .click();
      await page
        .getByRole("option", { name: "Office cash", exact: true })
        .click();
      await page
        .getByRole("button", {
          name: labels.retailPostingPreview + " (1)",
          exact: true,
        })
        .click();
      await page
        .getByRole("button", { name: labels.retailPostingConfirm, exact: true })
        .click();
      await expect(
        page.getByRole("row").filter({ hasText: "PR20260001" }),
      ).toContainText(labels.retailPosted);
      await page
        .getByLabel(labels.retailReason, { exact: true })
        .fill("Synthetic posting reversal");
      const postingLink = page.getByRole("link", {
        name: labels.retailPostingPdf,
        exact: true,
      });
      await expect(postingLink).toBeVisible();
      const postingUrl = new URL(
        (await postingLink.getAttribute("href"))!,
        page.url(),
      );
      const postingPdf = await page.request.get(
        `http://127.0.0.1:${port}${postingUrl.pathname}${postingUrl.search}`,
      );
      expect(postingPdf.ok()).toBe(true);
      expect((await postingPdf.body()).subarray(0, 4).toString()).toBe("%PDF");
      await page
        .getByRole("button", { name: labels.retailPostingReverse, exact: true })
        .click();
      await expect(
        page.getByRole("checkbox", {
          name: labels.retailSelect + " PR20260001",
        }),
      ).toBeEnabled();
      await page.screenshot({
        path: `docs/design/retail-posting-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await page.goto(`/#/companies/${alpha.id}/cash`);
      await page
        .getByRole("button", { name: labels.cashAccounts, exact: true })
        .click();
      const terminals = page.getByRole("region", {
        name: labels.terminalTitle,
        exact: true,
      });
      await terminals
        .getByLabel(labels.terminalCode, { exact: true })
        .fill("front_1");
      await terminals
        .getByLabel(labels.terminalName, { exact: true })
        .fill("Front desk");
      await terminals
        .getByRole("button", { name: labels.terminalAdd, exact: true })
        .click();
      const terminalRow = terminals
        .getByRole("row")
        .filter({ hasText: "FRONT_1" });
      await expect(terminalRow).toContainText("Front desk");
      await terminalRow
        .getByRole("button", { name: labels.terminalArchive, exact: true })
        .click();
      await expect(terminalRow).toContainText(labels.terminalArchived);
      await terminalRow
        .getByRole("button", { name: labels.terminalRestore, exact: true })
        .click();
      await expect(terminalRow).toContainText(labels.terminalActive);
      expect(await terminals.evaluate((el) => el.scrollLeft)).toBe(0);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await page.screenshot({
        path: `docs/design/sales-terminals-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page
        .getByRole("button", { name: labels.cashAccounts, exact: true })
        .click();
      await page
        .getByRole("button", { name: labels.cashManual, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.cashAmount, { exact: true })
        .fill("100.00");
      await dialog
        .getByLabel(labels.cashPartner, { exact: true })
        .fill("Synthetic buyer");
      await dialog
        .getByLabel(labels.cashPerson, { exact: true })
        .fill("Test Person");
      await dialog
        .getByLabel(labels.cashDescription, { exact: true })
        .fill("Cash payment");
      await dialog
        .getByRole("button", { name: labels.cashSave, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await page
        .getByRole("row")
        .filter({ hasText: "Synthetic buyer" })
        .getByRole("link", { name: /^PP2026/ })
        .click();
      const row = page.getByRole("row").filter({ hasText: issued.number });
      await row
        .getByRole("button", { name: labels.cashMatch, exact: true })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", { name: labels.cashMatch, exact: true })
        .click();
      await expect(page.getByRole("dialog")).not.toBeVisible();
      await expect(
        page.getByText(labels.cashCreatedPayment, { exact: true }).first(),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/cash-detail-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      const payments = await page.request.get(
        `${base}/invoices/${draft.id}/payments`,
      );
      expect((await payments.json()).summary.paid).toBe("100.00");
      await page
        .getByRole("button", { name: labels.cashUnmatch, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.cashReason, { exact: true })
        .fill("Synthetic correction");
      await dialog
        .getByRole("button", { name: labels.cashUnmatch, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await page
        .getByRole("button", { name: labels.cashCancel, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByLabel(labels.cashReason, { exact: true })
        .fill("Duplicate cash record");
      await dialog
        .getByRole("button", { name: labels.cashCancel, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByRole("button", { name: labels.cashCancel, exact: true }),
      ).toHaveCount(0);
      await page
        .getByRole("link", { name: labels.cashBack, exact: true })
        .click();
      await expect(
        page.getByRole("region", { name: labels.cashBalance }),
      ).toContainText(/1[\s\u00a0,]?000/);
      await page.screenshot({
        path: `docs/design/cash-register-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      const axe = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa"])
        .analyze();
      expect(axe.violations).toEqual([]);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      ).toBe(true);

      const automation = page.getByRole("region", {
        name: labels.cashAutomation,
      });
      await automation
        .getByRole("button", { name: new RegExp(labels.cashDefault) })
        .click();
      await page
        .getByRole("option", { name: "Office cash", exact: true })
        .click();
      await automation
        .getByRole("button", { name: labels.cashSave, exact: true })
        .click();
      await expect(
        automation.getByRole("button", { name: labels.cashSave, exact: true }),
      ).toBeDisabled();
      await expect(
        automation.getByText(labels.cashSaved, { exact: true }),
      ).toHaveText(labels.cashSaved);
      const current = await (
        await page.request.get(`${base}/invoices/${draft.id}/payments`)
      ).json();
      const today = new Intl.DateTimeFormat("en-CA", {
        timeZone: "Europe/Prague",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
      }).format(new Date());
      const paid = await post(`/invoices/${draft.id}/payments`, {
        request_id: crypto.randomUUID(),
        revision: current.summary.revision,
        payment: {
          amount: "75.00",
          paid_on: today,
          method: "cash",
          note: "Synthetic automatic cash",
        },
      });
      await page.reload();
      await page
        .getByRole("link", { name: "PP2026000003", exact: true })
        .click();
      await expect(
        page.getByText(labels.cashAutomaticSource, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("button", { name: labels.cashUnmatch, exact: true }),
      ).toHaveCount(0);
      await page
        .getByRole("button", { name: new RegExp(labels.cashPdfLanguage) })
        .click();
      await page
        .getByRole("option", { name: labels.taxEnglish, exact: true })
        .click();
      const downloadEvent = page.waitForEvent("download");
      await page
        .getByRole("button", { name: labels.cashPdf, exact: true })
        .click();
      const download = await downloadEvent;
      expect(download.suggestedFilename()).toBe("PP2026000003-en.pdf");
      const path = await download.path();
      expect(readFileSync(path!).subarray(0, 4).toString()).toBe("%PDF");
      await page
        .getByRole("button", { name: labels.cashPrint, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await expect(dialog.locator("iframe")).toHaveAttribute("src", /^blob:/);
      await page.keyboard.press("Escape");
      await page.screenshot({
        path: `docs/design/cash-automatic-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      const autoAxe = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa"])
        .analyze();
      expect(autoAxe.violations).toEqual([]);
      const active = paid.items.find(
        (p: { reversed_at: string | null }) => !p.reversed_at,
      );
      await post(`/invoices/${draft.id}/payments/${active.id}/reverse`, {
        request_id: crypto.randomUUID(),
        revision: paid.summary.revision,
        reason: "Synthetic reverse",
        replacement: null,
      });
      await page.reload();
      await expect(
        page.getByText(`${labels.cashCancelled} · Synthetic reverse`, {
          exact: true,
        }),
      ).toBeVisible();
      await page.goto(`/#/companies/${_beta.id}/cash`);
      await expect(
        page.getByText(labels.cashNoAccounts, { exact: true }),
      ).toBeVisible();
      const vatProfile = await makeProfile("VAT cash", true);
      const vatBase = `http://127.0.0.1:${port}/api/companies/${vatProfile.id}`;
      const acResponse = await page.request.post(`${vatBase}/cash/commands`, {
        headers: { Origin: "http://127.0.0.1:5173", "X-Faktura-Request": "1" },
        data: {
          request_id: crypto.randomUUID(),
          action: "account",
          id: null,
          revision: null,
          archived: false,
          data: {
            name: "VAT register",
            currency: "CZK",
            opening_balance: "0.00",
            opening_on: "2026-01-01",
          },
        },
      });
      expect(acResponse.ok()).toBe(true);
      await page.goto(`/#/companies/${vatProfile.id}/cash`);
      await page
        .getByRole("button", { name: labels.cashManual, exact: true })
        .click();
      dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", { name: new RegExp(labels.cashTaxKind) })
        .click();
      await page
        .getByRole("option", { name: labels.cashTaxStandalone, exact: true })
        .click();
      await dialog.getByLabel(labels.cashTaxPrice, { exact: true }).fill("100");
      await dialog
        .getByRole("button", { name: new RegExp(labels.cashTaxPriceMode) })
        .click();
      await page
        .getByRole("option", { name: labels.cashTaxNetPrice, exact: true })
        .click();
      await dialog
        .getByLabel(labels.cashDescription, { exact: true })
        .fill("Synthetic taxable service");
      await expect(dialog.getByText(/121[.,]00/)).toBeVisible();
      await dialog
        .getByRole("button", { name: labels.cashSave, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByRole("region", { name: labels.cashBalance }),
      ).toContainText(/121[.,]00/);
      await page
        .getByRole("link", { name: "PP2026000001", exact: true })
        .click();
      await expect(
        page.getByRole("heading", {
          name: labels.cashTaxBreakdown,
          exact: true,
        }),
      ).toBeVisible();
      await expect(
        page.getByRole("heading", { name: labels.cashMatchTitle, exact: true }),
      ).toHaveCount(0);
      await expect(page.getByText(/21[.,]00/).first()).toBeVisible();
      await page.screenshot({
        path: `docs/design/cash-tax-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
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
