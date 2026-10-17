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
  test(`contacts persist, archive and remain isolated (${english ? "en" : "cs"})`, async ({
    page,
  }, info) => {
    test.setTimeout(90000);
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
      await page.goto(`/#/companies/${alpha.id}/contacts`);
      await expect(
        page.getByText(text("Zatím žádní odběratelé", "No customers yet")),
      ).toBeVisible();
      await page
        .getByRole("link", {
          name: text("Nový odběratel", "New customer"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        })
        .fill("Studio Test");
      // Deterministic registry boundary: exercise real UI merge, retry and errors.
      let registryFails = true;
      await page.route("**/contacts/ares/*", async (route) => {
        await route.fulfill({
          status: registryFails ? 503 : 200,
          json: registryFails
            ? { code: "unavailable" }
            : {
                registration_number: "00006947",
                name: "Registry name",
                vat_number: "CZ00006947",
                billing_address: {
                  street: "Letenská 525/15",
                  city: "Praha",
                  postal_code: "11800",
                  country: "CZ",
                },
              },
        });
      });
      await page
        .getByLabel(text("IČO", "Registration number"), { exact: true })
        .fill("00006947");
      await page
        .getByLabel(text("IČO", "Registration number"), { exact: true })
        .press("Tab");
      await expect(
        page.getByText(
          text(
            "ARES je nyní nedostupný. Zkuste načtení znovu nebo údaje vyplňte ručně.",
            "ARES is unavailable. Retry or enter the details manually.",
          ),
        ),
      ).toBeVisible();
      registryFails = false;
      await page
        .getByRole("button", {
          name: text("Doplnit z ARES", "Fill from ARES"),
          exact: true,
        })
        .click();
      await expect(
        page.getByLabel(text("Ulice a číslo", "Street and number"), {
          exact: true,
        }),
      ).toHaveValue("Letenská 525/15");
      await expect(
        page.getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        }),
      ).toHaveValue("Studio Test");
      await page
        .getByLabel(text("E-mail", "Email"), { exact: true })
        .fill("billing@example.test");
      await page
        .getByLabel(text("Město", "City"), { exact: true })
        .fill("Praha");
      await page
        .getByRole("tab", {
          name: text("Dodací adresy", "Delivery addresses"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: text("Přidat dodací adresu", "Add delivery address"),
        })
        .click();
      await page
        .getByLabel(text("Označení adresy", "Address label"))
        .fill("Warehouse");
      await page
        .getByLabel(text("Město", "City"), { exact: true })
        .fill("Brno");
      await page
        .getByRole("button", {
          name: new RegExp(
            text("Výchozí dodací adresa", "Default delivery address"),
          ),
        })
        .click();
      await page.getByRole("option", { name: "Warehouse" }).click();
      await page
        .getByRole("tab", {
          name: text("Základní údaje", "Basic details"),
          exact: true,
        })
        .click();
      await page
        .getByLabel(
          text("Další e-mailoví příjemci", "Additional email recipients"),
          { exact: true },
        )
        .fill("copy@example.test");
      await page
        .getByRole("button", {
          name: text("Přidat kontaktní osobu", "Add contact person"),
        })
        .click();
      await page
        .getByLabel(text("Jméno", "Name"), { exact: true })
        .fill("Jana Test");
      await page
        .getByLabel(text("E-mail", "Email"), { exact: true })
        .last()
        .fill("jana@example.test");
      await page
        .getByRole("button", {
          name: text("Vytvořit odběratele", "Create customer"),
        })
        .click();
      await expect(page).toHaveURL(new RegExp(`/contacts/[0-9]+$`));
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Studio Test",
      );
      const contactUrl = page.url();
      await page
        .getByRole("tab", {
          name: text("Základní údaje", "Basic details"),
          exact: true,
        })
        .focus();
      await page.keyboard.press("ArrowRight");
      await expect(
        page.getByRole("tab", {
          name: text("Dodací adresy", "Delivery addresses"),
          exact: true,
        }),
      ).toHaveAttribute("aria-selected", "true");
      await expect(
        page.getByRole("heading", { name: "Warehouse", exact: true }),
      ).toBeVisible();
      await page.keyboard.press("ArrowLeft");
      await expect(
        page.getByRole("tab", {
          name: text("Základní údaje", "Basic details"),
          exact: true,
        }),
      ).toHaveAttribute("aria-selected", "true");
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page.reload();
      await expect(
        page.getByRole("button", {
          name: text("Upravit kontakt", "Edit contact"),
          exact: true,
        }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/contacts-detail-${english ? "en" : "cs"}-${info.project.name}.png`,
        animations: "disabled",
        fullPage: true,
      });
      const summaryColumn = await page
        .locator(".contact-facts dd")
        .first()
        .boundingBox();
      const summarySection = await page
        .locator(".contact-section")
        .first()
        .boundingBox();
      await page
        .getByRole("button", {
          name: text("Upravit kontakt", "Edit contact"),
          exact: true,
        })
        .click();
      if (info.project.name === "desktop") {
        const inputColumn = await page
          .getByLabel(text("Název nebo jméno", "Company or full name"), {
            exact: true,
          })
          .boundingBox();
        const editorSection = await page
          .locator(".contact-section")
          .first()
          .boundingBox();
        expect(Math.abs(inputColumn!.x - summaryColumn!.x)).toBeLessThan(1);
        expect(Math.abs(editorSection!.y - summarySection!.y)).toBeLessThan(1);
      }

      await expect(
        page.getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        }),
      ).toHaveValue("Studio Test");
      await page
        .getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        })
        .fill("Studio Updated");
      await page
        .getByRole("button", {
          name: text("Uložit změny", "Save changes"),
          exact: true,
        })
        .click();
      await expect(
        page
          .getByRole("status")
          .filter({ hasText: text("Kontakt byl uložen", "Contact saved") }),
      ).toContainText(text("Kontakt byl uložen", "Contact saved"));
      await page
        .getByRole("button", {
          name: text("Upravit kontakt", "Edit contact"),
          exact: true,
        })
        .click();
      const contactId = contactUrl.split("/").at(-1)!;
      const endpoint = `http://127.0.0.1:${port}/api/companies/${alpha.id}/contacts/${contactId}`;
      const latest = await (await page.request.get(endpoint)).json();
      const concurrent = await page.request.put(endpoint, {
        headers: { Origin: "http://127.0.0.1:5173", "X-Faktura-Request": "1" },
        data: {
          revision: latest.revision,
          archived: false,
          data: { ...latest.data, note: "Concurrent update" },
        },
      });
      expect(concurrent.ok()).toBe(true);
      await page
        .getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        })
        .fill("Unsaved version");
      await page
        .getByRole("button", {
          name: text("Uložit změny", "Save changes"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("alert")).toContainText(
        text("Kontakt se mezitím změnil", "The contact changed"),
      );
      await expect(
        page.getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        }),
      ).toHaveValue("Unsaved version");
      await page.getByRole("alert").getByRole("button").click();
      await page.getByRole("dialog").getByRole("button").last().click();
      await expect(
        page.getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        }),
      ).toHaveValue("Studio Updated");
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/contacts-editor-${english ? "en" : "cs"}-${info.project.name}.png`,
        animations: "disabled",
        fullPage: true,
      });
      // Cancel must preserve saved data and ask before discarding edits.
      await page
        .getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        })
        .fill("Discard this");
      await page
        .getByRole("button", { name: text("Zrušit", "Cancel"), exact: true })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Zahodit změny", "Discard changes"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        "Studio Updated",
      );
      await expect(page.getByRole("textbox")).toHaveCount(0);
      await page
        .getByRole("link", {
          name: text("Faktury odběratele", "Customer invoices"),
          exact: true,
        })
        .click();
      await expect(page).toHaveURL(new RegExp(`customer=${contactId}`));
      await page.goto(contactUrl);
      await page
        .getByRole("button", { name: text("Akce", "Actions"), exact: true })
        .click();
      await page
        .getByRole("menuitem", {
          name: text("Archivovat kontakt", "Archive contact"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Archivovat kontakt", "Archive contact"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", { name: text("Akce", "Actions"), exact: true })
        .click();
      await expect(
        page.getByRole("menuitem", {
          name: text("Obnovit kontakt", "Restore contact"),
          exact: true,
        }),
      ).toBeVisible();
      await page.keyboard.press("Escape");
      await page
        .getByRole("link", {
          name: text("Odběratelé", "Customers"),
          exact: true,
        })
        .last()
        .click();
      await expect(
        page.getByRole("link", { name: "Studio Updated", exact: true }),
      ).toHaveCount(0);
      await page
        .getByRole("button", {
          name: text("Archivované", "Archived"),
          exact: true,
        })
        .click();
      await page
        .getByRole("link", { name: "Studio Updated", exact: true })
        .click();
      await page
        .getByRole("button", { name: text("Akce", "Actions"), exact: true })
        .click();
      await page
        .getByRole("menuitem", {
          name: text("Obnovit kontakt", "Restore contact"),
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: text("Obnovit kontakt", "Restore contact"),
          exact: true,
        })
        .click();
      await page
        .getByRole("button", { name: text("Akce", "Actions"), exact: true })
        .click();
      await expect(
        page.getByRole("menuitem", {
          name: text("Archivovat kontakt", "Archive contact"),
          exact: true,
        }),
      ).toBeVisible();
      await page.keyboard.press("Escape");
      await page
        .getByRole("link", {
          name: text("Odběratelé", "Customers"),
          exact: true,
        })
        .last()
        .click();
      await page
        .getByLabel(
          text(
            "Hledat podle názvu, IČO nebo e-mailu",
            "Search by name, registration number or email",
          ),
        )
        .fill("Updated");
      await page
        .getByRole("button", { name: text("Hledat", "Search"), exact: true })
        .click();
      await expect(
        page.getByRole("link", { name: "Studio Updated", exact: true }),
      ).toBeVisible();
      await expect(page).toHaveURL(/q=Updated/);
      await expect(page.getByRole("status")).toHaveCount(0);
      await expect(
        page
          .getByRole("table")
          .getByRole("link", { name: "Studio Updated", exact: true }),
      ).toBeVisible();
      // Real pagination uses the server, including active search and profile scope.
      for (let index = 1; index <= 11; index++) {
        const added = await page.request.post(
          `http://127.0.0.1:${port}/api/companies/${alpha.id}/contacts`,
          {
            headers: {
              Origin: "http://127.0.0.1:5173",
              "X-Faktura-Request": "1",
            },
            data: {
              request_id: crypto.randomUUID(),
              data: {
                ...latest.data,
                name: `Studio Updated ${String(index).padStart(2, "0")}`,
                registration_number: "",
                vat_number: "",
                email: `studio${index}@example.test`,
                persons: [],
              },
            },
          },
        );
        expect(added.ok()).toBe(true);
      }
      await page
        .getByRole("button", {
          name: new RegExp(text("Položek na stránku", "Items per page")),
        })
        .click();
      const resized = page.waitForResponse(
        (r) =>
          r.url().includes("/contacts?") && r.url().includes("page_size=10"),
      );
      await page.getByRole("option", { name: "10", exact: true }).click();
      expect((await resized).ok()).toBe(true);
      await expect(page).toHaveURL(/size=10/);
      await expect(page.getByRole("table").locator("tbody tr")).toHaveCount(10);
      await page
        .getByRole("button", { name: text("Další", "Next"), exact: true })
        .click();
      await expect(page).toHaveURL(/page=2/);
      await expect(page.getByRole("table").locator("tbody tr")).toHaveCount(2);
      await page
        .getByRole("button", {
          name: text("Předchozí", "Previous"),
          exact: true,
        })
        .click();
      await expect(page.getByRole("table").locator("tbody tr")).toHaveCount(10);
      await page.reload();
      await expect(page.getByRole("table").locator("tbody tr")).toHaveCount(10);
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      ).toBe(true);

      await expect(page.getByRole("table")).toBeVisible();

      await page.getByRole("button", { name: "Export", exact: true }).click();
      const exportDialog = page.getByRole("dialog");
      await expect(exportDialog).toBeVisible();
      await exportDialog.screenshot({
        path: `docs/design/export-contacts-${english ? "en" : "cs"}-${info.project.name}.png`,
        animations: "disabled",
      });
      const exportDownload = page.waitForEvent("download");
      await exportDialog
        .getByRole("button", {
          name: text("Stáhnout export", "Download export"),
        })
        .click();
      expect((await exportDownload).suggestedFilename()).toBe(
        "open-faktura-contacts.xlsx",
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
        "open-faktura-contacts.csv",
      );
      expect((await csvResponse).ok()).toBe(true);
      await page.getByRole("table").screenshot({
        path: `docs/design/contacts-table-${english ? "en" : "cs"}-${info.project.name}.png`,
        animations: "disabled",
      });
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({
        path: `docs/design/contacts-list-${english ? "en" : "cs"}-${info.project.name}.png`,
        animations: "disabled",
        fullPage: true,
      });
      await page
        .getByRole("link", {
          name: text(
            "Upravit kontakt: Studio Updated",
            "Edit contact: Studio Updated",
          ),
          exact: true,
        })
        .click();
      await expect(
        page.getByLabel(text("Název nebo jméno", "Company or full name"), {
          exact: true,
        }),
      ).toHaveValue("Studio Updated");
      await page.goto(`/#/companies/${alpha.id}/contacts`);
      await page.getByRole("button", { name: /Alpha/ }).click();
      await page.getByRole("button", { name: "Beta", exact: true }).click();
      await expect(page).toHaveURL(
        new RegExp(`/companies/${beta.id}/contacts$`),
      );
      await expect(
        page.getByText(text("Zatím žádní odběratelé", "No customers yet")),
      ).toBeVisible();
      await page.goto(
        contactUrl.replace(`/companies/${alpha.id}/`, `/companies/${beta.id}/`),
      );
      await expect(page.getByRole("alert")).toContainText(
        text("nemáte přístup", "do not have access"),
      );
      sql(
        `UPDATE membership SET role='viewer' WHERE company_id=${Number(alpha.id)}`,
        database,
      );
      await page.goto(contactUrl);
      await expect(
        page.getByText(
          text(
            "Tento kontakt můžete pouze prohlížet.",
            "You have read-only access to this contact.",
          ),
        ),
      ).toBeVisible();
      await expect(page.getByRole("textbox")).toHaveCount(0);
      await expect(
        page.getByRole("button", {
          name: text("Upravit kontakt", "Edit contact"),
          exact: true,
        }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("button", {
          name: text("Archivovat kontakt", "Archive contact"),
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
      sql(`DROP DATABASE ${database} WITH (FORCE)`);
      rmSync(directory, { recursive: true, force: true });
    }
  });
