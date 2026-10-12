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
  test(`installation SMTP settings and test (${english ? "en" : "cs"})`, async ({
    page,
  }, info) => {
    test.setTimeout(120000);
    const database = `company_e2e_${randomBytes(8).toString("hex")}`;
    const directory = mkdtempSync(join(tmpdir(), "faktura-auth-"));
    const secret = randomBytes(32).toString("hex");
    writeFileSync(join(directory, "bootstrap"), secret, { mode: 0o600 });
    const port = 3900 + info.workerIndex;
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
      const open = async () => {
        await page
          .getByRole("button", {
            name: text("Uživatelské menu", "Account menu"),
          })
          .click();
        await page
          .getByRole("button", {
            name: text("Nastavení e-mailu (SMTP)", "Email settings (SMTP)"),
            exact: true,
          })
          .click();
        await expect(page.getByRole("dialog")).toBeVisible();
      };
      await open();
      await page
        .getByRole("button", {
          name: text("Způsob odesílání", "Delivery method"),
        })
        .click();
      await page
        .getByRole("option", {
          name: text("Vlastní SMTP", "Custom SMTP"),
          exact: true,
        })
        .click();
      await page.getByLabel("SMTP server", { exact: true }).fill("127.0.0.1");
      await page
        .getByRole("button", {
          name: text("Zabezpečení", "Security"),
        })
        .click();
      await page
        .getByRole("option", {
          name: text(
            "Bez TLS (pouze lokální server)",
            "No TLS (local server only)",
          ),
        })
        .click();
      await page
        .getByLabel("Port", { exact: true })
        .fill(String(smtpAddress.port));
      await page
        .getByLabel(text("E-mail odesílatele", "Sender email"), { exact: true })
        .fill("custom@example.test");
      await expect(
        page.getByRole("button", {
          name: text("Odeslat testovací e-mail", "Send test email"),
        }),
      ).toBeDisabled();
      await page
        .getByRole("button", {
          name: text("Uložit nastavení", "Save settings"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("status")).toContainText(
        text("Nastavení uloženo", "Settings saved"),
      );
      await page.getByRole("dialog").evaluate(async (el) => {
        await Promise.all(el.getAnimations().map((a) => a.finished));
      });
      expect(
        (await new AxeBuilder({ page }).analyze()).violations.filter(
          (v) => v.impact === "serious" || v.impact === "critical",
        ),
      ).toEqual([]);
      await page.getByRole("dialog").screenshot({
        path: `docs/design/smtp-${english ? "en" : "cs"}-${info.project.name}.png`,
      });
      await page
        .getByLabel(text("Adresa pro test", "Test recipient"), { exact: true })
        .fill("test@example.test");
      await page.route("**/installation/smtp/test", async (route) => {
        const response = await route.fetch({
          url: `http://127.0.0.1:${port}/api/installation/smtp/test`,
        });
        expect(response.ok()).toBeTruthy();
        await route.abort("failed");
        await page.unroute("**/installation/smtp/test");
      });
      await page
        .getByRole("button", {
          name: text("Odeslat testovací e-mail", "Send test email"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("alert")).toContainText(
        text("Nastavení nebo test", "Could not complete"),
      );
      await page
        .getByRole("button", {
          name: text("Odeslat testovací e-mail", "Send test email"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(
          text("Poštovní server zprávu přijal", "The mail server accepted"),
          { exact: false },
        ),
      ).toBeVisible();
      expect(messages).toHaveLength(1);
      expect(messages[0]).toContain("custom@example.test");
      await page
        .getByRole("dialog")
        .getByRole("button", { name: text("Zavřít", "Close"), exact: true })
        .last()
        .click();
      await open();
      await expect(page.getByLabel("SMTP server", { exact: true })).toHaveValue(
        "127.0.0.1",
      );
      await page
        .getByRole("button", {
          name: text("Způsob odesílání", "Delivery method"),
        })
        .click();
      await page
        .getByRole("option", { name: text("Vypnuto", "Disabled"), exact: true })
        .click();
      await page
        .getByRole("button", {
          name: text("Uložit nastavení", "Save settings"),
          exact: true,
        })
        .click();
      await expect(
        page.getByRole("button", {
          name: text("Odeslat testovací e-mail", "Send test email"),
          exact: true,
        }),
      ).toBeDisabled();
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
