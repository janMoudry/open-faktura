import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";
import { createServer } from "node:net";
async function unusedPort(): Promise<number> {
  const listener = createServer();
  await new Promise<void>((resolve, reject) => {
    listener.once("error", reject);
    listener.listen(0, "127.0.0.1", resolve);
  });
  const address = listener.address();
  if (!address || typeof address === "string")
    throw new Error("Missing test port");
  const port = address.port;
  await new Promise<void>((resolve, reject) =>
    listener.close((error) => (error ? reject(error) : resolve())),
  );
  return port;
}
async function waitForExit(
  server: ChildProcess,
  timeout: number,
): Promise<boolean> {
  if (server.exitCode !== null || server.signalCode !== null) return true;
  return new Promise((resolve) => {
    const done = () => {
      clearTimeout(timer);
      resolve(true);
    };
    const timer = setTimeout(() => {
      server.removeListener("exit", done);
      resolve(false);
    }, timeout);
    server.once("exit", done);
  });
}
for (const english of [false, true])
  test(`nonpayer advances retain different rates (${english ? "en" : "cs"})`, async ({
    page,
    baseURL,
  }, info) => {
    test.setTimeout(90000);
    page.setDefaultTimeout(15000);
    page.setDefaultNavigationTimeout(15000);
    const origin = new URL(baseURL!).origin;
    const database = `company_e2e_${randomBytes(8).toString("hex")}`;
    const directory = mkdtempSync(join(tmpdir(), "faktura-advance-fx-"));
    const secret = randomBytes(32).toString("hex");
    writeFileSync(join(directory, "bootstrap"), secret, { mode: 0o600 });
    const port = await unusedPort();
    const backend = `http://127.0.0.1:${port}`;
    const sql = (query: string, target = "postgres") =>
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
          target,
          "-v",
          "ON_ERROR_STOP=1",
          "-c",
          query,
        ],
        { stdio: "pipe", timeout: 10000 },
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
          CORS_ORIGINS: origin,
          PUBLIC_WEB_URL: origin,
        },
        stdio: "ignore",
      });
      let spawnError: Error | undefined;
      server.once("error", (error) => {
        spawnError = error;
      });
      await expect
        .poll(
          async () => {
            if (spawnError) throw spawnError;
            if (server!.exitCode !== null || server!.signalCode !== null)
              throw new Error(
                `Test API exited before readiness: ${server!.exitCode ?? server!.signalCode}`,
              );
            return fetch(`${backend}/api/ready`, {
              signal: AbortSignal.timeout(1000),
            })
              .then((r) => r.status)
              .catch(() => 0);
          },
          { timeout: 20000 },
        )
        .toBe(200);
      const post = async (path: string, data: unknown) => {
        const r = await page.request.post(backend + path, {
          headers: { Origin: origin, "X-Faktura-Request": "1" },
          data,
        });
        expect(r.ok(), `POST ${path} ${r.status()}`).toBe(true);
        return r.json().catch(() => null);
      };
      await post("/api/auth/setup", {
        secret,
        email: "fx@example.test",
        password: "Synthetic password 123!",
      });
      await post("/api/auth/login", {
        email: "fx@example.test",
        password: "Synthetic password 123!",
      });
      const settings = {
        name: "Nonpayer FX",
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
        tax_effective_from: "2026-01-01",
        currency: "CZK",
        timezone: "Europe/Prague",
        maturity_days: 14,
        payment_method: "bank_transfer",
        iban: "",
        number_prefix: "FV",
        number_year: 2026,
        next_number: 1,
      };
      const company = await post("/api/companies", {
        request_id: crypto.randomUUID(),
        settings,
      });
      const sources = [];
      for (const [price, rate] of [
        ["1000", "25"],
        ["500", "24"],
      ]) {
        const invoice = JSON.parse(
          readFileSync("tests/fixtures/invoice-draft.json", "utf8"),
        );
        invoice.supplier.name = settings.name;
        invoice.supplier.registration_number = settings.registration_number;
        invoice.calculation.tax_mode = "non_payer";
        invoice.calculation.currency = "EUR";
        invoice.calculation.exchange_rate = { rate, units: "1" };
        invoice.calculation.lines[0].vat_percent = "0";
        invoice.calculation.lines[0].quantity = "1";
        invoice.calculation.lines[0].unit_price = price;
        invoice.payment.method = "bank_transfer";
        invoice.payment.bank_account = "12345678/0100";
        invoice.tax_on = null;
        const source = await post(`/api/companies/${company.id}/sales`, {
          request_id: crypto.randomUUID(),
          kind: "advance",
          data: {
            name: `FX advance ${rate}`,
            maturity_days: 14,
            status: "created",
            invoice,
          },
        });
        await post(
          `/api/companies/${company.id}/advances/${source.id}/payments`,
          {
            request_id: crypto.randomUUID(),
            revision: source.revision,
            payment: {
              amount: price,
              paid_on: "2026-10-12",
              method: "bank_transfer",
              note: "Synthetic advance payment",
            },
          },
        );
        sources.push(source);
      }
      const draft = await post(
        `/api/companies/${company.id}/advance-settlements`,
        {
          request_id: crypto.randomUUID(),
          invoice_id: null,
          revision: null,
          advances: sources.map((s) => ({
            id: s.id,
            revision: s.revision + 1,
          })),
        },
      );
      await page.addInitScript(
        (lang) => localStorage.setItem("open-faktura.language", lang),
        english ? "en" : "cs",
      );
      await page.route("**/api/**", (route) => {
        const u = new URL(route.request().url());
        return route.continue({ url: backend + u.pathname + u.search });
      });
      await page.goto(`/#/companies/${company.id}/invoices/${draft.id}`);
      const text = (cs: string, en: string) => (english ? en : cs);
      await expect(
        page.getByText(
          text(
            "Původní kurz zálohy: 1 EUR = 25 CZK",
            "Original advance rate: 1 EUR = 25 CZK",
          ),
          { exact: true },
        ),
      ).toBeVisible();
      await expect(
        page.getByText(
          text(
            "Původní kurz zálohy: 1 EUR = 24 CZK",
            "Original advance rate: 1 EUR = 24 CZK",
          ),
          { exact: true },
        ),
      ).toBeVisible();
      await page
        .getByLabel(text("Kurz v CZK", "Rate in CZK"), { exact: true })
        .fill("26");
      await page
        .getByLabel(text("Jednotková cena", "Unit price"), { exact: true })
        .first()
        .fill("1100");
      const saved = page.waitForResponse(
        (r) =>
          r.request().method() === "PUT" &&
          new URL(r.url()).pathname ===
            `/api/companies/${company.id}/invoices/${draft.id}`,
      );
      await page
        .getByRole("button", {
          name: text("Uložit návrh", "Save draft"),
          exact: true,
        })
        .click();
      expect((await saved).ok()).toBe(true);
      await page.reload();
      await expect(
        page.getByLabel(text("Kurz v CZK", "Rate in CZK"), { exact: true }),
      ).toHaveValue("26");
      await page
        .getByRole("button", {
          name: text("Vystavit fakturu", "Issue invoice"),
          exact: true,
        })
        .click();
      const issuing = page.waitForResponse(
        (r) =>
          r.request().method() === "POST" &&
          new URL(r.url()).pathname ===
            `/api/companies/${company.id}/invoices/${draft.id}/issue`,
        { timeout: 15000 },
      );
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Vystavit fakturu", "Issue invoice"),
          exact: true,
        })
        .click();
      const issueResponse = await issuing;
      const issueBody = await issueResponse.text();
      expect(
        issueResponse.ok(),
        `Issue ${issueResponse.status()}: ${issueBody}`,
      ).toBe(true);
      expect(
        () => JSON.parse(issueBody),
        "Issue response must be JSON",
      ).not.toThrow();
      await expect(page.getByRole("heading", { level: 1 })).toContainText(
        /FV2026[0-9]+/,
      );
      const response = await page.request.get(
        `${backend}/api/companies/${company.id}/invoices/${draft.id}`,
      );
      expect(response.ok()).toBe(true);
      const issued = await response.json();
      expect(issued.totals.payable).toBe("100.00");
      expect(issued.data.calculation.exchange_rate.rate).toBe("26");
      expect(
        issued.advances.sources.map(
          (s: { valuation: { exchange_rate: { rate: string } } }) =>
            s.valuation.exchange_rate.rate,
        ),
      ).toEqual(["25", "24"]);
      for (const source of sources) {
        const r = await page.request.get(
          `${backend}/api/companies/${company.id}/sales/${source.id}`,
        );
        expect((await r.json()).data.invoice.calculation.exchange_rate).toEqual(
          source.data.invoice.calculation.exchange_rate,
        );
      }
      await page.screenshot({
        path: info.outputPath("nonpayer-fx-issued.png"),
        fullPage: true,
      });
    } catch (error) {
      await page
        .screenshot({
          path: info.outputPath("nonpayer-fx-failure.png"),
          fullPage: true,
          timeout: 3000,
        })
        .catch(() => {});
      throw error;
    } finally {
      // Do not wait for interrupted route handlers during Playwright timeout cleanup.
      await page.unrouteAll({ behavior: "ignoreErrors" }).catch(() => {});
      if (server && server.exitCode === null && server.signalCode === null) {
        server.kill("SIGINT");
        if (!(await waitForExit(server, 1000))) {
          server.kill("SIGKILL");
          if (!(await waitForExit(server, 1000)))
            console.error(`Test API ${server.pid} did not exit after SIGKILL`);
        }
      }
      try {
        sql(`DROP DATABASE ${database} WITH (FORCE)`);
      } finally {
        rmSync(directory, { recursive: true, force: true });
      }
    }
  });
