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
for (const currency of ["CZK", "EUR"])
  for (const english of [false, true])
    test(`credit notes, setoff and refunds (${english ? "en" : "cs"}, ${currency})`, async ({
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
      await new Promise<void>((resolve) =>
        smtp.listen(0, "127.0.0.1", resolve),
      );
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
        await page
          .getByLabel("Zopakujte heslo")
          .fill("Synthetic password 123!");
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
          beta = await makeProfile("Beta");
        const draftData = JSON.parse(
          readFileSync("tests/fixtures/invoice-draft.json", "utf8"),
        );
        draftData.supplier = {
          ...draftData.supplier,
          name: "Alpha",
          registration_number: "12345678",
          vat_number: "CZ12345678",
        };
        draftData.payment.method =
          currency === "CZK" ? "cash" : "bank_transfer";
        if (currency !== "CZK") {
          draftData.payment.bank_account = "12345678/0100";
          draftData.calculation.currency = currency;
          draftData.calculation.exchange_rate = { rate: "25", units: "1" };
        }
        draftData.calculation.tax_mode = "vat_payer";
        draftData.calculation.price_mode = "net";
        draftData.calculation.lines[0].unit_price = "1000";
        draftData.calculation.lines[0].quantity = "1";
        draftData.calculation.lines[0].vat_percent = "21";
        draftData.language = english ? "en" : "cs";
        draftData.tax_on = draftData.issued_on;
        const headers = {
          Origin: "http://127.0.0.1:5173",
          "X-Faktura-Request": "1",
        };
        const api = `http://127.0.0.1:${port}/api/companies/${alpha.id}`;
        const made = await page.request.post(`${api}/invoices`, {
          headers,
          data: { request_id: crypto.randomUUID(), data: draftData },
        });
        expect(made.ok()).toBe(true);
        const invoice = await made.json();
        const issued = await page.request.post(
          `${api}/invoices/${invoice.id}/issue`,
          {
            headers,
            data: {
              request_id: crypto.randomUUID(),
              revision: invoice.revision,
            },
          },
        );
        expect(issued.ok()).toBe(true);
        await page.goto(`/#/companies/${alpha.id}/invoices/${invoice.id}`);
        await page
          .getByRole("button", {
            name: text("Vytvořit dobropis", "Create credit note"),
            exact: true,
          })
          .click();
        await expect(page).toHaveURL(/credit-notes\/\d+$/);
        const creditId = page.url().split("/").at(-1)!;
        await page
          .getByLabel(text("Zdůvodnění opravy", "Reason for correction"), {
            exact: true,
          })
          .fill("Synthetic partial return");
        await page
          .getByLabel(
            text("Částka opravy položky 1", "Correction amount for item 1"),
            { exact: true },
          )
          .fill("605");
        await page
          .getByRole("button", {
            name: text("Uložit návrh", "Save draft"),
            exact: true,
          })
          .click();
        await expect(
          page.getByRole("button", {
            name: text("Vystavit dobropis", "Issue credit note"),
            exact: true,
          }),
        ).toBeEnabled();
        await page.evaluate(() => {
          window.scrollTo(0, 0);
          document.querySelectorAll<HTMLElement>("*").forEach((element) => {
            if (element.scrollWidth > element.clientWidth)
              element.scrollLeft = 0;
          });
        });
        await page.screenshot({
          path: `docs/design/credit-editor-${currency}-${english ? "en" : "cs"}-${info.project.name}.png`,
          fullPage: true,
        });
        await page
          .getByRole("button", {
            name: text("Vystavit dobropis", "Issue credit note"),
            exact: true,
          })
          .click();
        await page
          .getByRole("dialog")
          .getByRole("button", {
            name: text("Vystavit dobropis", "Issue credit note"),
            exact: true,
          })
          .click();
        await expect(page.getByRole("heading", { level: 1 })).toHaveText(
          "DB20260001",
          { timeout: 20000 },
        );
        const issuedCredit = await (
          await page.request.get(`${api}/credit-notes/${creditId}`)
        ).json();
        expect(issuedCredit.totals.currency).toBe(currency);
        expect(issuedCredit.totals.amounts.vat).toBe("-105.00");
        expect(issuedCredit.totals.payable).toBe("-605.00");
        const downloadEvent = page.waitForEvent("download");
        await page
          .getByRole("button", {
            name: text("Stáhnout PDF", "Download PDF"),
            exact: true,
          })
          .click();
        expect((await downloadEvent).suggestedFilename()).toBe(
          "DB20260001.pdf",
        );
        const pdf = await page.request.get(
          `${api}/credit-notes/${creditId}/pdf`,
        );
        expect(pdf.ok()).toBe(true);
        writeFileSync(
          `output/credit-${currency}-${english ? "en" : "cs"}-${info.project.name}.pdf`,
          await pdf.body(),
        );
        await page
          .getByRole("button", {
            name: text("Započíst s fakturou", "Offset against invoice"),
            exact: true,
          })
          .click();
        await page
          .getByRole("dialog")
          .getByRole("button", {
            name: text("Započíst s fakturou", "Offset against invoice"),
            exact: true,
          })
          .click();
        await expect(page.getByRole("dialog")).toHaveCount(0);
        let actual = await (
          await page.request.get(`${api}/invoices/${invoice.id}`)
        ).json();
        expect(actual.settlement.outstanding).toBe("605.00");
        await page
          .getByRole("button", {
            name: text("Stornovat záznam", "Reverse entry"),
            exact: true,
          })
          .click();
        await page
          .getByRole("dialog")
          .getByLabel(text("Zdůvodnění opravy", "Reason for correction"), {
            exact: true,
          })
          .fill("Synthetic reversal");
        await page
          .getByRole("dialog")
          .getByRole("button", {
            name: text("Stornovat záznam", "Reverse entry"),
            exact: true,
          })
          .click();
        await expect(page.getByRole("dialog")).toHaveCount(0);
        actual = await (
          await page.request.get(`${api}/invoices/${invoice.id}`)
        ).json();
        expect(actual.settlement.outstanding).toBe("1210.00");
        await page
          .getByRole("button", {
            name: text("Zapsat vrácení peněz", "Record money refund"),
            exact: true,
          })
          .click();
        await page
          .getByRole("dialog")
          .getByLabel(
            `${text("Vrácená částka", "Refunded amount")} (${currency})`,
            { exact: true },
          )
          .fill("605");
        await page
          .getByRole("dialog")
          .getByLabel(text("Zdůvodnění opravy", "Reason for correction"), {
            exact: true,
          })
          .fill("Synthetic cash refund");
        await page
          .getByRole("dialog")
          .getByRole("button", {
            name: text("Zapsat vrácení peněz", "Record money refund"),
            exact: true,
          })
          .click();
        await expect(page.getByRole("dialog")).toHaveCount(0);
        const final = await (
          await page.request.get(`${api}/credit-notes/${creditId}`)
        ).json();
        expect(final.remaining).toBe("0.00");
        expect(final.events).toHaveLength(3);
        await page.evaluate(() => {
          window.scrollTo(0, 0);
          document.querySelectorAll<HTMLElement>("*").forEach((element) => {
            if (element.scrollWidth > element.clientWidth)
              element.scrollLeft = 0;
          });
        });
        await page.screenshot({
          path: `docs/design/credit-detail-${currency}-${english ? "en" : "cs"}-${info.project.name}.png`,
          fullPage: true,
        });
        await page.goto(`/#/companies/${alpha.id}/credit-notes`);
        await expect(
          page.getByRole("link", { name: "DB20260001", exact: true }),
        ).toBeVisible();
        expect((await new AxeBuilder({ page }).analyze()).violations).toEqual(
          [],
        );
        await page.screenshot({
          path: `docs/design/credit-list-${currency}-${english ? "en" : "cs"}-${info.project.name}.png`,
          fullPage: true,
        });
        await page.goto(`/#/companies/${beta.id}/credit-notes`);
        await expect(
          page.getByText(
            text("Zatím nemáte žádné dobropisy.", "No credit notes yet."),
            { exact: true },
          ),
        ).toBeVisible();
      } catch (error) {
        await page.screenshot({
          path: `test-results/credit-failure-${english ? "en" : "cs"}-${info.project.name}.png`,
          fullPage: true,
        });
        throw error;
      } finally {
        await page.unrouteAll({ behavior: "wait" }).catch(() => {});
        if (server && server.exitCode === null) {
          const exited = new Promise((resolve) =>
            server!.once("exit", resolve),
          );
          server.kill("SIGINT");
          await exited;
        }
        await new Promise<void>((resolve) => smtp.close(() => resolve()));
        sql(`DROP DATABASE ${database} WITH (FORCE)`);
        rmSync(directory, { recursive: true, force: true });
      }
    });
