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
  test(`invoice drafts persist, copy and remain isolated (${english ? "en" : "cs"})`, async ({
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
      await page.goto(`/#/companies/${alpha.id}/invoices`);
      await expect(
        page.getByRole("heading", {
          name: text("Zatím žádné faktury", "No invoices yet"),
        }),
      ).toBeVisible();
      await page
        .getByRole("link", {
          name: text("Nová faktura", "New invoice"),
          exact: true,
        })
        .last()
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        text("Nová faktura", "New invoice"),
      );
      await expect(
        page.getByRole("tab", {
          name: text("Doklad", "Document"),
          exact: true,
        }),
      ).toHaveAttribute("aria-selected", "true");
      await expect(
        page.getByRole("tab", {
          name: text("Přílohy", "Attachments"),
          exact: true,
        }),
      ).not.toHaveAttribute("aria-disabled", "true");
      await expect(
        page.getByRole("heading", {
          name: text("Datumy", "Dates"),
          exact: true,
        }),
      ).toBeVisible();
      await expect(
        page.getByRole("heading", {
          name: text("Moje platební údaje", "My payment details"),
          exact: true,
        }),
      ).toBeVisible();
      await page
        .getByLabel(text("Číslo objednávky", "Order number"), { exact: true })
        .fill("ORDER-UI");
      await page.screenshot({
        path: `docs/design/invoice-create-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });

      await expect(
        page
          .getByRole("textbox", {
            name: text("Název nebo jméno", "Company or full name"),
            exact: true,
          })
          .last(),
      ).not.toBeVisible();
      await page
        .getByRole("button", {
          name: text(
            "Zadat odběratele ručně / podle IČO",
            "Enter customer manually / by registration number",
          ),
          exact: true,
        })
        .click();
      await page
        .getByRole("textbox", {
          name: text("Název nebo jméno", "Company or full name"),
          exact: true,
        })
        .last()
        .fill("Synthetic buyer");
      await page
        .getByRole("button", {
          name: text(
            "Uložit odběratele do adresáře",
            "Save customer to directory",
          ),
          exact: true,
        })
        .click();
      await expect(
        page.getByRole("button", {
          name: text(
            "Uložit odběratele do adresáře",
            "Save customer to directory",
          ),
          exact: true,
        }),
      ).toHaveCount(0);
      const picker = page.getByRole("combobox", {
        name: text(
          "Najít odběratele v adresáři",
          "Find a customer in the directory",
        ),
      });
      await picker.evaluate((el) =>
        el.scrollIntoView({ block: "center", behavior: "instant" }),
      );
      await picker.evaluate(
        () =>
          new Promise<void>((resolve) =>
            requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
          ),
      );
      if (info.project.name === "mobile") await picker.tap();
      else await picker.click();
      // Focus alone must show the initial page, including the just-saved contact.
      await expect(
        page.getByRole("option", { name: /Synthetic buyer/ }),
      ).toBeVisible();
      await expect(picker).toHaveValue("");
      await picker.fill("Synthetic");
      // React Aria defers mobile popover reveal while the software keyboard settles.
      // Playwright visibility ignores opacity; wait before it scrolls/clicks the option.
      await expect(page.locator(".ui-popover")).toHaveCSS("opacity", "1");
      await expect(
        page.getByRole("option", { name: /Synthetic buyer/ }),
      ).toBeVisible();
      if (info.project.name === "mobile")
        await page.getByRole("option", { name: /Synthetic buyer/ }).tap();
      else await page.getByRole("option", { name: /Synthetic buyer/ }).click();
      await page
        .getByLabel(text("Popis položky", "Item description"), { exact: true })
        .fill("Design services");
      const unit = page.getByRole("button", {
        name: new RegExp(text("Jednotka", "Unit")),
      });
      await unit.click();
      await page
        .getByRole("option", {
          name: text("hod · hodina", "hod · hour"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: text("Uložit návrh", "Save draft"),
          exact: true,
        })
        .click();
      await expect(page).toHaveURL(/invoices\/\d+$/);
      const draftUrl = page.url();
      const draftId = draftUrl.split("/").at(-1)!;
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        text("Rozpracovaná faktura", "Invoice draft"),
      );
      await page.reload();
      await expect(
        page.getByLabel(text("Popis položky", "Item description"), {
          exact: true,
        }),
      ).toHaveValue("Design services");
      await expect(unit).toContainText(text("hod · hodina", "hod · hour"));
      await page
        .getByLabel(text("Množství", "Quantity"), { exact: true })
        .fill("3");
      // Hold an old calculation until a newer one has completed.
      let releaseOld!: () => void;
      const held = new Promise<void>((resolve) => {
        releaseOld = resolve;
      });
      let oldReady = false;
      let oldFinished = false;
      await page.route("**/invoices/calculate", async (route) => {
        const input = route.request().postDataJSON();
        if (input.lines[0]?.unit_price !== "2") return route.fallback();
        const response = await page.request.post(
          `http://127.0.0.1:${port}/api/companies/${alpha.id}/invoices/calculate`,
          {
            headers: {
              Origin: "http://127.0.0.1:5173",
              "X-Faktura-Request": "1",
            },
            data: input,
          },
        );
        oldReady = true;
        await held;
        await route.fulfill({ response }).catch(() => {});
        oldFinished = true;
      });
      await page
        .getByLabel(text("Jednotková cena", "Unit price"), { exact: true })
        .fill("2");
      await expect.poll(() => oldReady).toBe(true);
      await page
        .getByLabel(text("Jednotková cena", "Unit price"), { exact: true })
        .fill("19.995");
      await expect(
        page
          .getByRole("region", {
            name: text("Rekapitulace", "Summary"),
            exact: true,
          })
          .getByText(text("59,99 Kč", "59.99 CZK"), { exact: true }),
      ).toBeVisible();
      releaseOld();
      await expect.poll(() => oldFinished).toBe(true);
      await expect(
        page
          .getByRole("region", {
            name: text("Rekapitulace", "Summary"),
            exact: true,
          })
          .getByText(text("59,99 Kč", "59.99 CZK"), { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", {
          name: text("Uložit návrh", "Save draft"),
          exact: true,
        })
        .click();
      await expect(
        page.getByText(
          text("Návrh faktury byl uložen.", "Invoice draft saved."),
          { exact: true },
        ),
      ).toBeVisible();
      const headers = {
        Origin: "http://127.0.0.1:5173",
        "X-Faktura-Request": "1",
      };
      const endpoint = `http://127.0.0.1:${port}/api/companies/${alpha.id}/invoices/${draftId}`;
      const existing = await (await page.request.get(endpoint)).json();
      expect(
        (
          await page.request.put(endpoint, {
            headers,
            data: {
              revision: existing.revision,
              data: { ...existing.data, note: "Concurrent edit" },
            },
          })
        ).ok(),
      ).toBe(true);
      await page
        .getByLabel(text("Popis položky", "Item description"), { exact: true })
        .fill("My unsaved version");
      await page
        .getByRole("button", {
          name: text("Uložit návrh", "Save draft"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("alert")).toContainText(
        text("Vaše úpravy zůstaly zachované", "Your edits have been preserved"),
      );
      await expect(
        page.getByLabel(text("Popis položky", "Item description"), {
          exact: true,
        }),
      ).toHaveValue("My unsaved version");
      await page.getByRole("alert").getByRole("button").click();
      await page.getByRole("dialog").getByRole("button").last().click();
      await expect(
        page.getByLabel(text("Popis položky", "Item description"), {
          exact: true,
        }),
      ).toHaveValue("Design services");
      if (info.project.name === "desktop") {
        const sidebar = page.locator(".workspace-sidebar");
        const before = await sidebar.boundingBox();
        await page.evaluate(() =>
          window.scrollTo(0, document.body.scrollHeight),
        );
        await expect
          .poll(() => page.evaluate(() => window.scrollY))
          .toBeGreaterThan(0);
        const after = await sidebar.boundingBox();
        expect(after!.y).toBe(before!.y);
        expect(after!.height).toBe(before!.height);
        expect(after!.y + after!.height).toBeLessThanOrEqual(
          page.viewportSize()!.height,
        );
        await expect(page.getByRole("banner")).toBeInViewport();
        await page.evaluate(() => window.scrollTo(0, 0));
      }
      if (info.project.name === "desktop") {
        const description = await page
          .getByLabel(text("Popis položky", "Item description"), {
            exact: true,
          })
          .boundingBox();
        const unitControl = await page
          .getByRole("button", { name: new RegExp(text("Jednotka", "Unit")) })
          .boundingBox();
        expect(Math.abs(description!.y - unitControl!.y)).toBeLessThan(2);
      }
      const axe = await new AxeBuilder({ page }).analyze();
      expect(axe.violations).toEqual([]);
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/invoice-editor-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page
        .getByRole("link", {
          name: text("Kopírovat návrh", "Copy draft"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        text("Kopie faktury", "Invoice copy"),
      );
      await page
        .getByRole("button", {
          name: text("Uložit návrh", "Save draft"),
          exact: true,
        })
        .click();
      await expect(page).toHaveURL(/invoices\/\d+$/);
      expect(page.url()).not.toBe(draftUrl);
      await page
        .getByRole("link", {
          name: text("Faktury", "Invoices"),
          exact: true,
        })
        .last()
        .click();
      await expect(page.getByRole("table")).toContainText("Synthetic buyer");
      await page
        .getByLabel(
          text(
            "Hledat odběratele, objednávku nebo návrh",
            "Search customer, order or draft",
          ),
        )
        .fill("Synthetic buyer");
      await page
        .getByRole("button", { name: text("Hledat", "Search"), exact: true })
        .click();
      await expect(page).toHaveURL(/q=Synthetic/);
      await expect(page.getByRole("table").getByRole("row")).toHaveCount(3);
      await page
        .getByRole("button", { name: text("Filtry", "Filters"), exact: true })
        .click();
      const filterPicker = page.getByRole("combobox", {
        name: text(
          "Najít odběratele v adresáři",
          "Find a customer in the directory",
        ),
      });
      await filterPicker.evaluate((el) =>
        el.scrollIntoView({ block: "center", behavior: "instant" }),
      );
      await filterPicker.evaluate(
        () =>
          new Promise<void>((resolve) =>
            requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
          ),
      );
      await filterPicker.focus();
      await expect(page.locator(".ui-popover")).toHaveCSS("opacity", "1");
      const filteredInvoices = page.waitForResponse(
        (response) =>
          response.url().includes("customer_id=") &&
          response.request().method() === "GET" &&
          response.status() === 200,
      );
      await page.getByRole("option", { name: /Synthetic buyer/ }).click();
      await filteredInvoices;
      await expect(page).toHaveURL(/customer=\d+/);
      await expect(page.getByRole("table").getByRole("row")).toHaveCount(3);
      await expect(page.getByRole("table")).toBeVisible();
      await page.getByRole("table").screenshot({
        path: `docs/design/invoice-table-${english ? "en" : "cs"}-${info.project.name}.png`,
        animations: "disabled",
      });
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/invoice-list-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      const filters = page.locator("#invoice-filters");
      await filters
        .getByLabel(text("Cena od (Kč)", "Minimum amount (CZK)"))
        .fill("60");
      await filters
        .getByRole("button", {
          name: text("Filtrovat", "Apply filters"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("table")).toHaveCount(0);
      await expect(page).toHaveURL(/amount_min=60/);
      await page.reload();
      await expect(page.getByRole("table")).toHaveCount(0);
      await page.getByRole("button", { name: /^(Filtry|Filters)/ }).click();
      await filters
        .getByLabel(text("Cena od (Kč)", "Minimum amount (CZK)"))
        .fill("59.99");
      await filters
        .getByLabel(text("Cena do (Kč)", "Maximum amount (CZK)"))
        .fill("59.98");
      await filters
        .getByRole("button", {
          name: text("Filtrovat", "Apply filters"),
          exact: true,
        })
        .click();
      await expect(filters.getByRole("alert")).toBeVisible();
      await filters
        .getByLabel(text("Cena do (Kč)", "Maximum amount (CZK)"))
        .fill("59.99");
      await filters
        .getByRole("button", {
          name: text("Filtrovat", "Apply filters"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("table").getByRole("row")).toHaveCount(3);
      await page
        .getByRole("columnheader", { name: /^(Cena|Amount)/ })
        .getByRole("button")
        .click();
      await expect(page).toHaveURL(/sort=amount_asc/);
      await expect(
        page.getByRole("columnheader", { name: /^(Cena|Amount)/ }),
      ).toHaveAttribute("aria-sort", "ascending");
      await page.screenshot({
        path: `docs/design/invoice-filters-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page.getByRole("button", { name: /^(Filtry|Filters)/ }).click();
      await expect(page.getByRole("table")).toBeVisible();
      await page.screenshot({
        path: `docs/design/invoice-list-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page.getByRole("button", { name: /Alpha/ }).click();
      await page.getByRole("button", { name: "Beta", exact: true }).click();
      await expect(page).toHaveURL(
        new RegExp(`/companies/${beta.id}/invoices$`),
      );
      await expect(
        page.getByRole("heading", {
          name: text("Zatím žádné faktury", "No invoices yet"),
        }),
      ).toBeVisible();
      await page.goto(
        draftUrl.replace(`/companies/${alpha.id}/`, `/companies/${beta.id}/`),
      );
      await expect(page.getByRole("alert")).toContainText(
        text("nemáte přístup", "do not have access"),
      );
      sql(
        `UPDATE membership SET role='viewer' WHERE company_id=${Number(alpha.id)}`,
        database,
      );
      await page.goto(draftUrl);
      await expect(
        page.getByText(
          text(
            "Tento návrh můžete pouze prohlížet.",
            "You have read-only access to this draft.",
          ),
        ),
      ).toBeVisible();
      await expect(
        page.getByRole("button", {
          name: text("Uložit návrh", "Save draft"),
          exact: true,
        }),
      ).toHaveCount(0);
      // P07: validate saved data, issue once, reload immutable detail and download PDF.
      sql(
        `UPDATE membership SET role='owner' WHERE company_id=${Number(alpha.id)}`,
        database,
      );
      await page.goto(draftUrl);
      await page.reload();
      await page
        .getByRole("button", {
          name: text("Vystavit fakturu", "Issue invoice"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Vystavit fakturu", "Issue invoice"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("alert")).toContainText(
        text("IČO", "Registration"),
      );
      const currentResponse = await page.request.get(
        `http://127.0.0.1:${port}/api/companies/${alpha.id}/invoices/${draftId}`,
      );
      const current = await currentResponse.json();
      current.data.supplier.registration_number = "12345678";
      current.data.customer.address = {
        street: "Test 2",
        city: "Praha",
        postal_code: "11000",
        country: "CZ",
      };
      const updated = await page.request.put(
        `http://127.0.0.1:${port}/api/companies/${alpha.id}/invoices/${draftId}`,
        {
          headers: {
            Origin: "http://127.0.0.1:5173",
            "X-Faktura-Request": "1",
          },
          data: { revision: current.revision, data: current.data },
        },
      );
      expect(updated.ok()).toBe(true);
      await page.reload();
      await page
        .getByRole("button", {
          name: text("Vystavit fakturu", "Issue invoice"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Vystavit fakturu", "Issue invoice"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toContainText(
        "FV2026000001",
      );
      await page.reload();
      await expect(page.getByRole("heading", { level: 1 })).toContainText(
        "FV2026000001",
      );
      await expect(
        page.getByRole("button", {
          name: text("Uložit návrh", "Save draft"),
          exact: true,
        }),
      ).toHaveCount(0);
      const downloadPromise = page.waitForEvent("download");
      await page
        .getByRole("button", {
          name: text("Stáhnout PDF", "Download PDF"),
          exact: true,
        })
        .click();
      const download = await downloadPromise;
      expect(download.suggestedFilename()).toBe("FV2026000001.pdf");
      expect(await download.failure()).toBeNull();

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
        path: `docs/design/attachments-invoice-${english ? "en" : "cs"}-${info.project.name}.png`,
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
      await expect(page.getByRole("dialog")).toContainText(
        text("Odeslání faktury e-mailem", "Send invoice by email"),
      );
      await page
        .getByLabel(text("Komu", "To"), { exact: true })
        .fill("customer@example.test");
      await page.getByRole("checkbox", { name: /delivery-note.txt/ }).check();
      await expect(
        page.getByLabel(text("Předmět", "Subject"), { exact: true }),
      ).toHaveValue(/FV2026000001/);
      await expect(
        page.getByLabel(text("Text e-mailu", "Message"), { exact: true }),
      ).not.toHaveValue(/{{/);
      await page.getByRole("dialog").evaluate(async (element) => {
        await Promise.all(
          element.getAnimations().map((animation) => animation.finished),
        );
      });
      expect(
        (await new AxeBuilder({ page }).analyze()).violations.filter(
          (v) => v.impact === "serious" || v.impact === "critical",
        ),
      ).toEqual([]);
      await page.getByRole("dialog").screenshot({
        path: `docs/design/invoice-email-${english ? "en" : "cs"}-${info.project.name}.png`,
      });
      // An interrupted response must preserve the original idempotency key when retried.
      await page.route("**/invoices/*/deliveries", async (route) => {
        if (route.request().method() === "POST") {
          const response = await route.fetch({
            url: `http://127.0.0.1:${port}${new URL(route.request().url()).pathname}`,
          });
          expect(response.ok()).toBeTruthy();
          await route.abort("failed");
          await page.unroute("**/invoices/*/deliveries");
        } else await route.fallback();
      });
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Odeslat e-mailem", "Send by email"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("dialog").getByRole("alert")).toBeVisible();
      await expect(
        page.getByLabel(text("Komu", "To"), { exact: true }),
      ).toHaveValue("customer@example.test");
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

      expect(messages[0]).toContain("FV2026000001.pdf");
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
      const historical = await historicalDownload;
      expect(await historical.failure()).toBeNull();
      await page.keyboard.press("Escape");
      await page
        .locator(".invoice-sheet")
        .filter({
          has: page.getByRole("heading", {
            name: text("Historie odesílání", "Email history"),
            exact: true,
          }),
        })
        .screenshot({
          path: `docs/design/invoice-email-history-${english ? "en" : "cs"}-${info.project.name}.png`,
        });
      const pdfButton = page.getByRole("button", {
        name: text("Stáhnout PDF", "Download PDF"),
        exact: true,
      });
      await expect(pdfButton).toBeEnabled();
      await pdfButton.evaluate(async (el) => {
        await Promise.allSettled(el.getAnimations().map((a) => a.finished));
      });
      // Audit stable button colours after async loading/disabled transitions.
      await page.locator("button").evaluateAll(async (buttons) => {
        await Promise.allSettled(
          buttons.flatMap((button) =>
            button.getAnimations().map((animation) => animation.finished),
          ),
        );
      });
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page.screenshot({
        path: `docs/design/invoice-issued-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page
        .getByRole("link", { name: text("Faktury", "Invoices"), exact: true })
        .last()
        .click();
      await page
        .getByRole("button", {
          name: text("K úhradě", "Outstanding"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("table").getByRole("row")).toHaveCount(2);
      await expect(page.getByRole("table")).toContainText("FV2026000001");
      await page.screenshot({
        path: `docs/design/invoice-list-issued-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page
        .getByRole("link", { name: "FV2026000001", exact: true })
        .click();
      await page
        .getByRole("tab", { name: text("Úhrady", "Payments"), exact: true })
        .click();
      const payments = page
        .getByRole("region", {
          name: text("Úhrady", "Payments"),
          exact: true,
        })
        .first();
      async function addPayment(amount: string) {
        await payments
          .getByRole("button", {
            name: text("Přidat úhradu", "Add payment"),
            exact: true,
          })
          .click();
        await page
          .getByRole("dialog")
          .getByLabel(text("Částka (CZK)", "Amount (CZK)"))
          .fill(amount);
        await page
          .getByRole("dialog")
          .getByRole("button", {
            name: text("Potvrdit úhradu", "Confirm payment"),
          })
          .click();
        await expect(page.getByRole("dialog")).toHaveCount(0);
      }
      await addPayment("20.00");
      await expect(payments).toContainText(
        text("Částečně uhrazeno", "Partially paid"),
      );
      await expect(payments).toContainText(text("39,99 Kč", "39.99 CZK"));
      await payments
        .getByRole("button", {
          name: text("Opravit úhradu", "Correct payment"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByLabel(text("Částka (CZK)", "Amount (CZK)"))
        .fill("10.00");
      await page
        .getByRole("dialog")
        .getByLabel(
          text("Důvod opravy nebo storna", "Reason for correction or reversal"),
        )
        .fill("Synthetic correction");
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Potvrdit úhradu", "Confirm payment"),
        })
        .click();
      await expect(page.getByRole("dialog")).toHaveCount(0);
      await expect(payments).toContainText("Synthetic correction");
      await expect(payments).toContainText(text("49,99 Kč", "49.99 CZK"));
      await addPayment("49.99");
      await expect(
        payments.getByText(text("Uhrazeno", "Paid"), { exact: true }),
      ).toBeVisible();
      await addPayment("5.00");
      await expect(
        payments.getByText(text("Přeplaceno", "Overpaid"), { exact: true }),
      ).toBeVisible();
      const overpayment = payments
        .getByRole("row")
        .filter({ hasText: text("5,00 Kč", "5.00 CZK") });
      await overpayment
        .getByRole("button", {
          name: text("Stornovat úhradu", "Reverse payment"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByLabel(
          text("Důvod opravy nebo storna", "Reason for correction or reversal"),
        )
        .fill("Synthetic reversal");
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Potvrdit úhradu", "Confirm payment"),
        })
        .click();
      await expect(page.getByRole("dialog")).toHaveCount(0);
      await page.reload();
      await expect(
        payments.getByText(text("Uhrazeno", "Paid"), { exact: true }),
      ).toBeVisible();
      await expect(payments).toContainText("Synthetic reversal");
      await expect(payments).toContainText("Synthetic correction");
      // Audit stable button colours after async loading/disabled transitions.
      await page.locator("button").evaluateAll(async (buttons) => {
        await Promise.allSettled(
          buttons.flatMap((button) =>
            button.getAnimations().map((animation) => animation.finished),
          ),
        );
      });
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page.screenshot({
        path: `docs/design/invoice-payments-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page.goto(`/#/companies/${alpha.id}/invoices?status=paid`);
      await page.getByRole("button", { name: "Export", exact: true }).click();
      const exportDialog = page.getByRole("dialog");
      await expect(exportDialog).toBeVisible();
      await exportDialog.screenshot({
        path: `docs/design/export-invoices-${english ? "en" : "cs"}-${info.project.name}.png`,
        animations: "disabled",
      });
      const exportDownload = page.waitForEvent("download");
      await exportDialog
        .getByRole("button", {
          name: text("Stáhnout export", "Download export"),
        })
        .click();
      expect((await exportDownload).suggestedFilename()).toBe(
        "open-faktura-invoices.xlsx",
      );
      await expect(exportDialog).toHaveCount(0);
      await page.getByRole("button", { name: "Export", exact: true }).click();
      await exportDialog.getByRole("button", { name: /Formát|Format/ }).click();
      await page
        .getByRole("option", { name: "CSV (.csv)", exact: true })
        .click();
      const csvResponse = page.waitForResponse(
        (r) => r.url().endsWith("/exports") && r.request().method() === "POST",
      );
      const csvDownload = page.waitForEvent("download");
      await exportDialog
        .getByRole("button", {
          name: text("Stáhnout export", "Download export"),
        })
        .click();
      expect((await csvDownload).suggestedFilename()).toBe(
        "open-faktura-invoices.csv",
      );
      expect((await csvResponse).ok()).toBe(true);
      await page.goto(`/#/companies/${alpha.id}/overview`);
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        text("Přehled", "Overview"),
      );
      await expect(page.getByRole("main")).toContainText(
        text("59,99 Kč", "59.99 CZK"),
      );
      // Audit stable button colours after async loading/disabled transitions.
      await page.locator("button").evaluateAll(async (buttons) => {
        await Promise.allSettled(
          buttons.flatMap((button) =>
            button.getAnimations().map((animation) => animation.finished),
          ),
        );
      });
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page.screenshot({
        path: `docs/design/overview-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      await page.goto(`/#/companies/${beta.id}/overview`);
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        text("Přehled", "Overview"),
      );
      await expect(page.getByRole("main")).not.toContainText(
        text("59,99 Kč", "59.99 CZK"),
      );
      sql(
        `UPDATE membership SET role='viewer' WHERE company_id=${Number(alpha.id)}`,
        database,
      );
      await page.goto(draftUrl);
      await page
        .getByRole("tab", { name: text("Úhrady", "Payments"), exact: true })
        .click();
      await expect(
        payments.getByText(text("Uhrazeno", "Paid"), { exact: true }),
      ).toBeVisible();
      await expect(
        payments.getByRole("button", {
          name: text("Přidat úhradu", "Add payment"),
          exact: true,
        }),
      ).toHaveCount(0);
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
