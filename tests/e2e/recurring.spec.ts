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
  test(`recurring schedule, automatic issue and mail (${english ? "en" : "cs"})`, async ({
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
      draftData.payment.method = "cash";
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
          data: { request_id: crypto.randomUUID(), revision: invoice.revision },
        },
      );
      expect(issued.ok()).toBe(true);

      await page.goto(`/#/companies/${alpha.id}/invoices/${invoice.id}`);
      await page
        .getByRole("link", {
          name: text(
            "Vytvořit pravidelnou fakturu",
            "Create recurring invoice",
          ),
          exact: true,
        })
        .click();
      await expect(page).toHaveURL(/recurring\/new/);
      await page
        .getByLabel(text("Název vzoru", "Template name"), { exact: true })
        .fill("Monthly support");
      await page
        .getByRole("button", {
          name: new RegExp(text("Ukončení", "Ends") + "$"),
        })
        .click();
      await page
        .getByRole("option", {
          name: text("Po počtu faktur", "After invoice count"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Celkový počet faktur", "Total invoice count"), {
          exact: true,
        })
        .fill("1");
      await page
        .getByRole("switch", {
          name: text(
            "Automaticky vystavovat (jinak vytvořit návrh)",
            "Issue automatically (otherwise create a draft)",
          ),
          exact: true,
        })
        .press("Space");
      await page
        .getByRole("switch", {
          name: text(
            "Po vystavení odeslat e-mail s PDF",
            "Send email with PDF after issue",
          ),
          exact: true,
        })
        .press("Space");
      await page
        .getByLabel(text("Příjemce e-mailu", "Email recipient"), {
          exact: true,
        })
        .fill("synthetic@example.test");
      await page
        .getByRole("button", {
          name: text("Uložit vzor", "Save template"),
          exact: true,
        })
        .click();
      await expect(page).toHaveURL(/recurring\/\d+$/);
      const ruleId = page.url().split("/").at(-1)!;
      await expect(
        page.getByText(text("Pozastaveno", "Paused"), { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("button", {
          name: text("Aktivovat opakování", "Activate schedule"),
          exact: true,
        }),
      ).toBeEnabled();
      await expect(
        page.getByText(text("Přepočítávám…", "Calculating…"), { exact: true }),
      ).not.toBeVisible();
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/recurring-editor-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("button", {
          name: text("Aktivovat opakování", "Activate schedule"),
          exact: true,
        })
        .click();
      const dialog = page.getByRole("dialog");
      await dialog
        .getByRole("button", {
          name: text("Potvrdit a aktivovat", "Confirm and activate"),
          exact: true,
        })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect
        .poll(
          async () => {
            const r = await page.request.get(`${api}/recurring/${ruleId}`);
            return (await r.json()).status;
          },
          { timeout: 20000 },
        )
        .toBe("completed");
      await expect.poll(() => messages.length, { timeout: 30000 }).toBe(1);
      expect(messages[0]).toContain("application/pdf");
      await page
        .getByRole("button", {
          name: text("Obnovit stav", "Refresh status"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(text("Dokončeno", "Completed"), { exact: true }),
      ).toBeVisible();
      const record = await (
        await page.request.get(`${api}/recurring/${ruleId}`)
      ).json();
      expect(record.generated).toBe(1);
      expect(record.occurrences).toHaveLength(1);
      const linked = record.occurrences[0];
      await expect(
        page.getByRole("link", { name: linked.number, exact: true }),
      ).toBeVisible();
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/recurring-detail-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page
        .getByRole("link", { name: linked.number, exact: true })
        .click();
      await expect(page).toHaveURL(
        new RegExp(`invoices/${linked.invoice_id}$`),
      );
      await page.goto(`/#/companies/${alpha.id}/recurring`);
      await expect(
        page.getByRole("link", { name: "Monthly support", exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/recurring-list-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page.goto(`/#/companies/${beta.id}/recurring`);
      await expect(
        page.getByText(
          text(
            "Zatím nemáte žádné pravidelné faktury.",
            "No recurring invoices yet.",
          ),
          { exact: true },
        ),
      ).toBeVisible();

      // New empty template stays paused; activation checks the actual invoice contents.
      await page
        .getByRole("link", {
          name: text("Nová pravidelná faktura", "New recurring invoice"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Název vzoru", "Template name"), { exact: true })
        .fill("Incomplete template");
      await page
        .getByRole("button", {
          name: text("Uložit vzor", "Save template"),
          exact: true,
        })
        .click();
      await expect(page).toHaveURL(/recurring\/\d+$/);
      await page
        .getByRole("button", {
          name: text("Aktivovat opakování", "Activate schedule"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Potvrdit a aktivovat", "Confirm and activate"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("dialog").getByRole("alert")).toContainText(
        text("Zkontrolujte povinné údaje", "Check required fields"),
      );
      const emptyRule = await (
        await page.request.get(
          `http://127.0.0.1:${port}/api/companies/${beta.id}/recurring/${page.url().split("/").at(-1)}`,
        )
      ).json();
      expect(emptyRule.status).toBe("paused");
      expect(emptyRule.generated).toBe(0);
      expect(emptyRule.data.auto_issue).toBe(false);
      expect(emptyRule.data.mail).toBeNull();
    } catch (error) {
      await page.screenshot({
        path: `test-results/recurring-failure-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
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
