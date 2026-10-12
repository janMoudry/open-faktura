import { createServer } from "node:net";
import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

test.use({ actionTimeout: 15000 });

// Each test uses a disposable DB/server, never bootstraps the developer's account.
for (const english of [false, true])
  test(`templates, quotes and conversion (${english ? "en" : "cs"})`, async ({
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

      await page.goto(`/#/companies/${alpha.id}/templates/new`);
      await page
        .getByLabel(text("Název šablony", "Template name"), { exact: true })
        .fill("Monthly consulting");
      await page
        .getByText(text("Údaje odběratele", "Customer details"), {
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        })
        .filter({ visible: true })
        .fill("Synthetic customer");
      await page
        .getByLabel(text("Popis položky", "Item description"), { exact: true })
        .fill("Consulting");
      await page
        .getByLabel(text("Jednotková cena", "Unit price"), { exact: true })
        .fill("1200");
      await page
        .getByLabel(text("Množství", "Quantity"), { exact: true })
        .fill("2");
      await page
        .getByRole("button", { name: text("Uložit", "Save"), exact: true })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Monthly consulting",
      );
      await page
        .getByRole("link", {
          name: text("Použít pro nabídku", "Use for quotation"),
          exact: true,
        })
        .click();
      await expect(
        page.getByLabel(text("Popis nabídky", "Quotation description"), {
          exact: true,
        }),
      ).toHaveValue("Monthly consulting");
      await page
        .getByRole("button", { name: text("Uložit", "Save"), exact: true })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "CN20260001",
      );
      await expect(page).toHaveURL(/quotes\/\d+$/);
      await expect(
        page.getByRole("button", {
          name: text("Vytvořit fakturu", "Create invoice"),
          exact: true,
        }),
      ).toBeVisible();
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/sales-quote-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await expect(page).toHaveURL(/quotes\/\d+$/);
      const downloadEvent = page.waitForEvent("download");
      await page
        .getByRole("button", {
          name: text("Stáhnout PDF", "Download PDF"),
          exact: true,
        })
        .click();
      const download = await downloadEvent;
      expect(download.suggestedFilename()).toBe("CN20260001.pdf");
      await page
        .getByRole("button", {
          name: text("Tisk / náhled", "Print / preview"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("dialog")).toBeVisible();
      await expect(
        page.getByRole("link", {
          name: text("Otevřít PDF v nové kartě", "Open PDF in a new tab"),
        }),
      ).toHaveAttribute("href", /^blob:/);
      await page.keyboard.press("Escape");
      await page
        .getByRole("tab", { name: text("Přílohy", "Attachments"), exact: true })
        .click();
      await page.locator('input[type="file"]').setInputFiles({
        name: "delivery-note.txt",
        mimeType: "text/plain",
        buffer: Buffer.from("Synthetic delivery note"),
      });
      await expect(
        page.getByText("delivery-note.txt", { exact: true }),
      ).toBeVisible();
      const ownDownload = page.waitForEvent("download");
      await page
        .getByRole("button", {
          name: text("Stáhnout", "Download"),
          exact: true,
        })
        .click();
      expect((await ownDownload).suggestedFilename()).toBe("delivery-note.txt");
      await page.getByRole("tabpanel").screenshot({
        path: `docs/design/attachments-quote-${english ? "en" : "cs"}-${info.project.name}.png`,
      });
      await page
        .getByRole("tab", { name: text("Doklad", "Document"), exact: true })
        .click();
      await page
        .getByRole("button", {
          name: text("Odeslat e-mailem", "Send by email"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Komu", "To"), { exact: true })
        .fill("customer@example.test");
      await page.getByRole("checkbox", { name: /delivery-note.txt/ }).check();
      await expect(
        page.getByLabel(text("Předmět", "Subject"), { exact: true }),
      ).toHaveValue(new RegExp(english ? "Quotation CN" : "Cenová nabídka CN"));
      const mailA11y = await new AxeBuilder({ page }).analyze();
      expect(
        mailA11y.violations.filter(
          (v) => v.impact === "critical" || v.impact === "serious",
        ),
      ).toEqual([]);
      await page.screenshot({
        path: `docs/design/quote-email-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Odeslat e-mailem", "Send by email"),
          exact: true,
        })
        .click();
      await expect(
        page
          .getByText(
            text("Předáno poštovnímu serveru", "Accepted by mail server"),
            { exact: true },
          )
          .filter({ visible: true }),
      ).toBeVisible();
      expect(messages).toHaveLength(1);
      expect(messages[0]).toContain("application/pdf");
      expect(messages[0]).toContain("delivery-note.txt");
      await page
        .getByRole("tab", { name: text("Přílohy", "Attachments"), exact: true })
        .click();
      page.once("dialog", (dialog) => dialog.accept());
      await page
        .getByRole("button", { name: text("Odstranit", "Remove"), exact: true })
        .click();
      await expect(
        page.getByText("delivery-note.txt", { exact: true }),
      ).toHaveCount(0);
      await page
        .getByRole("tab", { name: text("Doklad", "Document"), exact: true })
        .click();

      await page
        .getByRole("button", {
          name: text("Detail zprávy", "Message details"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("dialog")).toContainText(
        "customer@example.test",
      );
      const customHistory = page.waitForEvent("download");
      await page
        .getByRole("dialog")
        .getByRole("button", { name: "delivery-note.txt", exact: true })
        .click();
      expect((await customHistory).suggestedFilename()).toBe(
        "delivery-note.txt",
      );
      const historicalDownload = page.waitForEvent("download");
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Stáhnout PDF", "Download PDF"),
          exact: true,
        })
        .click();
      await historicalDownload;
      await page.keyboard.press("Escape");
      await page.screenshot({
        path: `docs/design/quote-history-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });

      await page
        .getByRole("button", {
          name: text("Vytvořit odkaz", "Create link"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Vytvořit odkaz", "Create link"),
          exact: true,
        })
        .click();
      const sharedUrl = await page
        .getByLabel(text("Odkaz na nabídku", "Quotation link"), { exact: true })
        .inputValue();
      expect(sharedUrl).toContain("/#/shared/quote/");
      const guest = await browser.newContext({
        viewport: page.viewportSize()!,
      });
      await guest.route("**/api/**", async (route) => {
        const url = new URL(route.request().url());
        await route.continue({
          url: `http://127.0.0.1:${port}${url.pathname}${url.search}`,
        });
      });
      const publicPage = await guest.newPage();
      try {
        await publicPage.goto(sharedUrl);
        await expect(
          publicPage.getByRole("heading", { level: 1 }),
        ).toContainText("CN20260001");
        await expect(
          publicPage.getByText("Consulting", { exact: true }),
        ).toBeVisible();
        expect(await guest.cookies()).toEqual([]);
        const downloaded = publicPage.waitForEvent("download");
        await publicPage
          .getByRole("button", {
            name: text("Stáhnout nabídku PDF", "Download quotation PDF"),
            exact: true,
          })
          .click();
        await downloaded;
        const publicA11y = await new AxeBuilder({ page: publicPage }).analyze();
        expect(
          publicA11y.violations.filter(
            (v) => v.impact === "critical" || v.impact === "serious",
          ),
        ).toEqual([]);
        await publicPage.screenshot({
          path: `docs/design/quote-public-${english ? "en" : "cs"}-${info.project.name}.png`,
          fullPage: true,
        });
        const shareStats = async () => {
          const response = await page.request.get(
            `http://127.0.0.1:${port}/api/companies/${alpha.id}/sales/${page.url().split("/").at(-1)}/shares`,
          );
          expect(response.ok()).toBeTruthy();
          return (await response.json()).items[0].view_count as number;
        };
        await expect.poll(shareStats).toBe(1);
        await publicPage.reload();
        await expect(
          publicPage.getByRole("heading", { level: 1 }),
        ).toContainText("CN20260001");
        await expect.poll(shareStats).toBe(2);
        await page
          .getByRole("button", {
            name: text("Obnovit", "Refresh"),
            exact: true,
          })
          .first()
          .click();
        await page.screenshot({
          path: `docs/design/quote-shares-${english ? "en" : "cs"}-${info.project.name}.png`,
          fullPage: true,
        });
        await page
          .getByRole("button", {
            name: text("Historie zobrazení", "View history"),
            exact: true,
          })
          .click();
        await expect(page.getByRole("dialog")).toContainText(
          text("Zobrazená verze", "Viewed version"),
        );
        await page.keyboard.press("Escape");
        await page
          .getByRole("button", {
            name: text("Odeslat odkaz", "Send link"),
            exact: true,
          })
          .click();
        await page
          .getByLabel(text("Komu", "To"), { exact: true })
          .fill("customer@example.test");
        await page
          .getByRole("dialog")
          .getByRole("button", {
            name: text("Odeslat e-mailem", "Send by email"),
            exact: true,
          })
          .click();
        await expect.poll(() => messages.length).toBe(2);
        expect(messages[1]).not.toContain("application/pdf");
        expect(messages[1].replace(/=\r\n/g, "")).toContain(
          sharedUrl.split("/").at(-1),
        );
        await page
          .getByRole("button", {
            name: text("Zrušit odkaz", "Revoke link"),
            exact: true,
          })
          .click();
        await page
          .getByRole("dialog")
          .getByRole("button", {
            name: text("Zrušit odkaz", "Revoke link"),
            exact: true,
          })
          .click();
        await expect(
          page.getByText(text("Zrušený", "Revoked"), { exact: true }),
        ).toBeVisible();
        await publicPage.reload();
        await expect(
          publicPage.getByRole("heading", { level: 1 }),
        ).toContainText(text("Nabídka není dostupná", "Quotation unavailable"));
      } finally {
        await guest.close();
      }
      const quoteUrl = page.url();
      await page
        .getByRole("button", {
          name: text("Vytvořit fakturu", "Create invoice"),
          exact: true,
        })
        .click();
      await expect(page).toHaveURL(/invoices\/\d+/);
      await expect(
        page.getByLabel(text("Popis položky", "Item description"), {
          exact: true,
        }),
      ).toHaveValue("Consulting");
      const invoiceUrl = page.url();
      const qid = quoteUrl.split("/").at(-1)!;
      const qr = await page.request.get(
        `http://127.0.0.1:${port}/api/companies/${alpha.id}/sales/${qid}`,
      );
      expect((await qr.json()).invoice_id).toBe(invoiceUrl.split("/").at(-1));
      await page.goto(quoteUrl);
      await page
        .getByRole("link", {
          name: text("Otevřít navazující fakturu", "Open linked invoice"),
        })
        .click();
      await expect(page).toHaveURL(invoiceUrl);
      await page.goto(`/#/companies/${alpha.id}/quotes`);
      await expect(
        page.getByRole("link", { name: "CN20260001", exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/sales-list-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      const a11y = await new AxeBuilder({ page }).analyze();
      expect(
        a11y.violations.filter(
          (v) => v.impact === "critical" || v.impact === "serious",
        ),
      ).toEqual([]);
      await page.goto(`/#/companies/${beta.id}/quotes`);
      await expect(
        page.getByText(
          text("Zatím tu nejsou žádné záznamy.", "There are no records yet."),
          { exact: true },
        ),
      ).toBeVisible();
    } catch (error) {
      await page.screenshot({
        path: `test-results/quote-failure-${english ? "en" : "cs"}-${info.project.name}.png`,
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
