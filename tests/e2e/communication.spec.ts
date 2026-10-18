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
  test(`reminders, payment thanks and settings (${english ? "en" : "cs"})`, async ({
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
      const draftData = JSON.parse(
        readFileSync("tests/fixtures/invoice-draft.json", "utf8"),
      );
      draftData.supplier = {
        ...draftData.supplier,
        name: "Alpha",
        registration_number: "12345678",
        vat_number: "CZ12345678",
      };
      draftData.payment.method = "cash";
      draftData.calculation.tax_mode = "vat_payer";
      draftData.calculation.price_mode = "net";
      draftData.calculation.lines[0].unit_price = "1000";
      draftData.calculation.lines[0].quantity = "1";
      draftData.calculation.lines[0].vat_percent = "21";
      draftData.language = english ? "en" : "cs";
      draftData.tax_on = draftData.issued_on;
      draftData.due_on = draftData.issued_on;
      draftData.customer.email = "customer@example.test";
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
          data: { request_id: crypto.randomUUID(), revision: invoice.revision },
        },
      );
      expect(issued.ok()).toBe(true);

      await page.goto(`/#/companies/${alpha.id}/invoices/${invoice.id}`);
      await page
        .getByRole("button", {
          name: text("Poslat upomínku", "Send reminder"),
          exact: true,
        })
        .click();
      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible();
      await expect(
        dialog.getByLabel(text("Předmět", "Subject"), { exact: true }),
      ).toHaveValue(new RegExp(english ? "Payment reminder" : "Upomínka"));
      await expect(dialog).toHaveCSS("opacity", "1");
      await page.screenshot({
        path: `docs/design/communication-compose-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await dialog
        .getByRole("button", {
          name: text("Odeslat e-mailem", "Send by email"),
          exact: true,
        })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect.poll(() => messages.length).toBe(1);
      const privacy = new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]);
      expect((await privacy.analyze()).violations).toEqual([]);
      await page
        .getByRole("switch", {
          name: text(
            "Pozastavit automatické e-maily u této faktury",
            "Pause automatic emails for this invoice",
          ),
        })
        .press("Space");
      await expect
        .poll(
          async () =>
            (
              await (
                await page.request.get(
                  `${api}/invoices/${invoice.id}/communication/reminder`,
                )
              ).json()
            ).paused,
        )
        .toBe(true);
      await page.goto(`/#/companies/${alpha.id}`);
      await page
        .getByRole("switch", {
          name: text(
            "Automaticky poděkovat po úplné úhradě",
            "Automatically thank after full payment",
          ),
        })
        .press("Space");
      await page
        .getByRole("button", {
          name: text("Uložit komunikaci", "Save communication settings"),
          exact: true,
        })
        .click();
      await expect(dialog).toBeVisible();
      await dialog
        .getByRole("button", {
          name: text(
            "Uložit a povolit odesílání",
            "Save and authorize sending",
          ),
          exact: true,
        })
        .click();
      await expect(dialog).not.toBeVisible();
      await page.reload();
      await expect(
        page.getByRole("switch", {
          name: text(
            "Automaticky poděkovat po úplné úhradě",
            "Automatically thank after full payment",
          ),
        }),
      ).toBeChecked();
      await page.screenshot({
        path: `docs/design/communication-settings-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page.goto(`/#/companies/${alpha.id}/invoices/${invoice.id}`);
      await page
        .getByRole("button", {
          name: text("Poděkovat za úhradu", "Thank for payment"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("alert")).toContainText(
        text(
          "Poděkování lze poslat po úplném uhrazení",
          "Thanks can be sent once the invoice is fully settled",
        ),
      );
      const settlement = await (
        await page.request.get(`${api}/invoices/${invoice.id}/payments`)
      ).json();
      const payment = await page.request.post(
        `${api}/invoices/${invoice.id}/payments`,
        {
          headers,
          data: {
            request_id: crypto.randomUUID(),
            revision: settlement.summary.revision,
            payment: {
              amount: "1210.00",
              paid_on: settlement.summary.today,
              method: "cash",
              note: "Synthetic payment",
            },
          },
        },
      );
      expect(payment.ok()).toBe(true);
      await page.reload();
      // Invoice pause also suppresses automatic thanks. Manual send remains explicit.
      await page
        .getByRole("button", {
          name: text("Poděkovat za úhradu", "Thank for payment"),
          exact: true,
        })
        .click();
      await expect(dialog).toBeVisible();
      await dialog
        .getByRole("button", {
          name: text("Odeslat e-mailem", "Send by email"),
          exact: true,
        })
        .click();
      await expect.poll(() => messages.length).toBe(2);
      await expect
        .poll(async () =>
          (
            await (
              await page.request.get(`${api}/invoices/${invoice.id}/deliveries`)
            ).json()
          ).items.every((v: { status: string }) => v.status === "sent"),
        )
        .toBe(true);
      await page
        .getByRole("button", { name: text("Obnovit", "Refresh"), exact: true })
        .click();
      await page.screenshot({
        path: `docs/design/communication-history-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      ).toBe(true);
    } catch (error) {
      await page.screenshot({
        path: `test-results/communication-failure-${english ? "en" : "cs"}-${info.project.name}.png`,
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
