import { createServer } from "node:net";
import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

test.use({ actionTimeout: 15000 });

// Deterministic synthetic PDF source, no customer data or external resources.
function syntheticPreviewPdf() {
  const content = "BT /F1 18 Tf 72 770 Td (Synthetic original preview) Tj ET\n";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    `<< /Length ${content.length} >>\nstream\n${content}endstream`,
  ];
  let pdf = "%PDF-1.4\n";
  const offsets = [0];
  objects.forEach((object, index) => {
    offsets.push(Buffer.byteLength(pdf));
    pdf += `${index + 1} 0 obj\n${object}\nendobj\n`;
  });
  const xref = Buffer.byteLength(pdf);
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets.slice(1))
    pdf += `${String(offset).padStart(10, "0")} 00000 n \n`;
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(pdf);
}

// Each test uses a disposable DB/server, never bootstraps the developer's account.
for (const english of [false, true])
  test(`received invoices (${english ? "en" : "cs"})`, async ({
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
      const labels = JSON.parse(
        readFileSync(
          `apps/web/src/locales/${english ? "en" : "cs"}.json`,
          "utf8",
        ),
      );
      const stockHeaders = {
        Origin: "http://127.0.0.1:5173",
        "X-Faktura-Request": "1",
      };
      const stockBase = `http://127.0.0.1:${port}/api/companies/${alpha.id}`;
      const stockCreated = await page.request.post(`${stockBase}/catalog`, {
        headers: stockHeaders,
        data: {
          request_id: crypto.randomUUID(),
          data: {
            name: "Physical goods",
            code: "GOODS",
            barcode: "",
            unit: "ks",
            quantity: "1",
            unit_price: "100",
            currency: "CZK",
            price_mode: "net",
            vat_percent: "21",
          },
        },
      });
      expect(stockCreated.ok()).toBe(true);
      const stockItem = await stockCreated.json();
      expect(
        (
          await page.request.post(`${stockBase}/stock/commands`, {
            headers: stockHeaders,
            data: {
              request_id: crypto.randomUUID(),
              catalog_id: stockItem.id,
              action: "open",
              quantity: "0",
              booked_on: "2026-10-01",
              reason: "Synthetic opening",
            },
          })
        ).ok(),
      ).toBe(true);
      await page.goto(`/#/companies/${alpha.id}/inbox`);
      await page.getByLabel(labels.inboxUpload, { exact: true }).setInputFiles({
        name: "synthetic-inbox.pdf",
        mimeType: "application/pdf",
        buffer: Buffer.from("%PDF-1.4\nSynthetic inbox test"),
      });
      await expect(
        page.getByRole("link", { name: "synthetic-inbox.pdf", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("link", { name: labels.inboxCreate, exact: true })
        .click();
      await page
        .getByLabel(labels.receivedSupplierName, { exact: true })
        .fill("Synthetic vendor");
      await page
        .getByLabel(labels.receivedExternal, { exact: true })
        .fill("SUPPLIER-0001");
      await page
        .getByLabel(`${labels.receivedLine} 1`, { exact: true })
        .fill("Hosting");
      await page
        .getByLabel(`${labels.receivedNet} 1`, { exact: true })
        .fill("100.01");
      await page
        .getByLabel(`${labels.draftVat} 1`, { exact: true })
        .fill("21.02");
      await page
        .getByLabel(labels.receivedRounding, { exact: true })
        .fill("-0.03");
      await page
        .getByRole("button", {
          name: new RegExp(`${labels.stockReceiveItem} 1`),
        })
        .click();
      await page
        .getByRole("option", { name: "Physical goods", exact: true })
        .click();
      await page
        .getByLabel(`${labels.stockReceiveQuantity} 1 (ks)`, { exact: true })
        .fill("2.5");
      await page
        .getByLabel(`${labels.stockAcquisitionCost} 1`, { exact: true })
        .fill("10.1234");
      await page
        .getByLabel(labels.stockIncidentalCost, { exact: true })
        .fill("20");
      await page
        .getByRole("button", { name: labels.receivedSave, exact: true })
        .click();
      await expect(page).toHaveURL(/received-invoices\/\d+$/);
      await expect(page.getByRole("heading", { level: 1 })).toContainText("PF");
      expect(
        (await (await page.request.get(`${stockBase}/stock/cards`)).json())[0]
          .quantity,
      ).toBe("2.5");
      const valuation = await (
        await page.request.get(`${stockBase}/stock/report?valuation=automatic`)
      ).json();
      expect(valuation.known_value).toBe("45.31");
      expect(valuation.items[0].unit_cost).toBe("18.1234");
      const detailUrl = page.url();
      await page.goto(`http://127.0.0.1:5173/#/companies/${alpha.id}/stock`);
      await page
        .getByRole("button", { name: new RegExp(labels.stockValuationMethod) })
        .click();
      await page
        .getByRole("option", { name: labels.stockAutomaticMethod, exact: true })
        .click();
      const overview = page.getByRole("region", {
        name: labels.stockOverviewTable,
        exact: true,
      });
      await expect(
        overview.getByText(english ? "18.1234 CZK" : "18,1234 CZK", {
          exact: true,
        }),
      ).toBeVisible();
      await page
        .getByRole("button", {
          name: labels.stockValuationBreakdown,
          exact: true,
        })
        .click();
      await expect(
        page.getByText(labels.stockCorrectionStock, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByText(labels.stockCorrectionIssued, { exact: true }),
      ).toBeVisible();
      await page.screenshot({
        path: `docs/design/stock-acquisition-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
      });
      await page.goto(detailUrl);
      await expect(page.getByRole("heading", { level: 1 })).toContainText("PF");
      await page.locator('input[type="file"]').setInputFiles({
        name: "source.txt",
        mimeType: "text/plain",
        buffer: Buffer.from("Synthetic original document"),
      });
      await expect(page.getByText("source.txt", { exact: true })).toBeVisible();
      await page
        .getByRole("button", { name: labels.receivedEdit, exact: true })
        .click();
      await page
        .getByLabel(labels.receivedDescription, { exact: true })
        .fill("Updated source record");
      await page
        .getByRole("button", { name: labels.receivedSave, exact: true })
        .click();
      await expect(
        page.getByRole("button", { name: labels.receivedEdit, exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: labels.paymentAdd, exact: true })
        .click();
      const dialog = page.getByRole("dialog");
      await expect(
        dialog.getByLabel(labels.paymentAmount, { exact: true }),
      ).toHaveValue("121.00");
      await dialog
        .getByLabel(labels.paymentAmount, { exact: true })
        .fill("21.00");
      await dialog
        .getByRole("button", { name: labels.paymentAdd, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(labels.paymentPartial, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("button", { name: labels.receivedEdit, exact: true }),
      ).toHaveCount(0);
      await page.screenshot({
        path: `docs/design/received-detail-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      expect(
        (
          await new AxeBuilder({ page })
            .withTags(["wcag2a", "wcag2aa"])
            .analyze()
        ).violations,
      ).toEqual([]);
      await page
        .getByRole("button", { name: labels.paymentReverse, exact: true })
        .click();
      await dialog
        .getByLabel(labels.paymentReason, { exact: true })
        .fill("Synthetic correction");
      await dialog
        .getByRole("button", { name: labels.paymentReverse, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(labels.paymentReversed, { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: labels.receivedCancel, exact: true })
        .click();
      await dialog
        .getByLabel(labels.paymentReason, { exact: true })
        .fill("Synthetic duplicate");
      await dialog
        .getByRole("button", { name: labels.receivedCancel, exact: true })
        .click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByText(labels.receivedCancelled, { exact: true }),
      ).toBeVisible();
      await page.goto(`/#/companies/${alpha.id}/inbox`);
      const mailboxPanel = page.locator("section").filter({
        has: page.getByRole("heading", {
          name: labels.mailboxTitle,
          exact: true,
        }),
      });
      await expect(
        mailboxPanel.getByLabel(labels.mailboxAutomatic, { exact: true }),
      ).toBeVisible();
      await expect(
        mailboxPanel.getByRole("button", {
          name: labels.mailboxSync,
          exact: true,
        }),
      ).toBeDisabled();
      await mailboxPanel
        .getByLabel(labels.mailboxProvider, { exact: true })
        .fill("unknown-synthetic-provider");
      await mailboxPanel
        .getByRole("button", { name: labels.inboxAdapterSave, exact: true })
        .click();
      await expect(mailboxPanel.getByRole("alert")).toContainText(
        labels.mailboxFailed,
      );
      await expect(
        mailboxPanel.getByLabel(labels.mailboxProvider, { exact: true }),
      ).toHaveValue("unknown-synthetic-provider");
      await mailboxPanel
        .getByLabel(labels.mailboxProvider, { exact: true })
        .fill("");
      await mailboxPanel
        .getByRole("button", { name: labels.inboxAdapterSave, exact: true })
        .click();
      await expect(mailboxPanel.getByRole("status")).toContainText(
        labels.inboxAdapterSaved,
      );
      await page.getByLabel(labels.inboxEmailEnabled, { exact: true }).click();
      await page
        .getByRole("option", { name: labels.inboxAdapterEnabled, exact: true })
        .click();
      await page
        .locator("section")
        .filter({
          has: page.getByRole("heading", {
            name: labels.inboxAdapters,
            exact: true,
          }),
        })
        .getByRole("button", { name: labels.inboxAdapterSave, exact: true })
        .click();
      await expect(
        page
          .locator("section")
          .filter({
            has: page.getByRole("heading", {
              name: labels.inboxAdapters,
              exact: true,
            }),
          })
          .getByRole("status"),
      ).toContainText(labels.inboxAdapterSaved);
      const notificationBell = page.getByRole("button", {
        name: new RegExp(`^${labels.inboxNotifications}`),
      });
      await notificationBell.click();
      const notifications = page.getByRole("dialog", {
        name: labels.inboxNotifications,
        exact: true,
      });
      await notifications
        .getByLabel(labels.inboxNotificationsDocuments, { exact: true })
        .check();
      await notifications
        .getByRole("button", {
          name: labels.inboxNotificationsSave,
          exact: true,
        })
        .click();
      await expect(
        notifications.getByRole("button", {
          name: labels.inboxNotificationsSave,
          exact: true,
        }),
      ).toBeDisabled();
      await page.keyboard.press("Escape");
      const mail = await page.request.post(`${stockBase}/inbox/mail`, {
        headers: stockHeaders,
        data: {
          request_id: crypto.randomUUID(),
          message_id: "synthetic-e2e@example.test",
          sender: "sender@example.test",
          subject: "Synthetic email intake",
          attachments: [
            {
              request_id: crypto.randomUUID(),
              filename: "email-original.pdf",
              base64: Buffer.from(
                "%PDF-1.4\nSynthetic gateway attachment",
              ).toString("base64"),
            },
          ],
        },
      });
      expect(mail.ok()).toBe(true);
      await page.reload();
      await expect(
        page.getByText("Synthetic email intake", { exact: false }),
      ).toBeVisible();
      await expect(
        page.getByRole("link", { name: "email-original.pdf", exact: true }),
      ).toBeVisible();
      const dropped = await page.evaluateHandle(
        (text) => {
          const transfer = new DataTransfer();
          transfer.items.add(
            new File([text], "synthetic.isdoc", { type: "application/xml" }),
          );
          return transfer;
        },
        readFileSync("tests/fixtures/inbox/synthetic-basic.isdoc", "utf8"),
      );
      await page
        .getByRole("group", { name: labels.inboxDropzone, exact: true })
        .dispatchEvent("drop", { dataTransfer: dropped });
      await dropped.dispose();
      const inboxRow = page
        .getByRole("row")
        .filter({ hasText: "synthetic.isdoc" });
      await expect(inboxRow).toBeVisible();
      await notificationBell.click();
      await notifications
        .getByRole("button", {
          name: labels.inboxNotificationsRefresh,
          exact: true,
        })
        .click();
      await expect(
        notifications.getByRole("link").filter({ hasText: "synthetic.isdoc" }),
      ).toBeVisible();
      await notifications
        .getByRole("button", {
          name: labels.inboxNotificationsRead,
          exact: true,
        })
        .click();
      await expect(
        notifications.getByRole("button", {
          name: labels.inboxNotificationsRead,
          exact: true,
        }),
      ).toBeDisabled();
      await page.keyboard.press("Escape");
      await page.reload();
      await expect(notificationBell).toHaveAccessibleName(
        labels.inboxNotificationsCount.replace("{{count}}", "0"),
      );

      await inboxRow
        .getByRole("button", { name: labels.inboxHistory, exact: true })
        .click();
      await expect(page.getByRole("dialog")).toContainText(labels.inboxUpload);
      await page.keyboard.press("Escape");
      await page.screenshot({
        path: `docs/design/inbox-${english ? "en" : "cs"}-${info.project.name}.png`,
        fullPage: true,
        animations: "disabled",
      });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      expect(
        (
          await new AxeBuilder({ page })
            .withTags(["wcag2a", "wcag2aa"])
            .analyze()
        ).violations,
      ).toEqual([]);
      await inboxRow
        .getByRole("link", { name: labels.inboxCreate, exact: true })
        .click();
      await expect(
        page.getByRole("complementary", {
          name: labels.inboxOriginal,
          exact: true,
        }),
      ).toContainText("SYNTHETIC-001");
      await page
        .getByRole("button", { name: labels.inboxExtract, exact: true })
        .click();
      await expect(
        page.getByLabel(labels.receivedExternal, { exact: true }),
      ).toHaveValue("SYNTHETIC-001");
      await expect(
        page.getByLabel(`${labels.draftVat} 1`, { exact: true }),
      ).toHaveValue("21.02");
      await page
        .getByRole("button", { name: labels.inboxCreateSupplier, exact: true })
        .click();
      const supplierDialog = page.getByRole("dialog");
      await supplierDialog
        .getByRole("button", {
          name: labels.inboxConfirmCreateSupplier,
          exact: true,
        })
        .click();
      await expect(supplierDialog).not.toBeVisible();
      await expect(
        page.getByText(labels.inboxSupplierCreated, { exact: true }),
      ).toBeVisible();
      await page
        .getByLabel(labels.receivedSupplierName, { exact: true })
        .fill("Reviewed supplier snapshot");
      await page
        .getByRole("button", { name: labels.inboxCompareSupplier, exact: true })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", {
          name: labels.inboxKeepSupplierSnapshot,
          exact: true,
        })
        .click();
      await expect(
        page.getByLabel(labels.receivedSupplierName, { exact: true }),
      ).toHaveValue("Reviewed supplier snapshot");

      await page
        .getByRole("button", { name: labels.receivedSave, exact: true })
        .click();
      await expect(page).toHaveURL(/received-invoices\/\d+$/);
      const processedDocumentUrl = page.url();
      await page.goto(`/#/companies/${alpha.id}/inbox`);
      await inboxRow
        .getByRole("button", { name: labels.inboxDetach, exact: true })
        .click();
      const detachDialog = page.getByRole("dialog", {
        name: labels.inboxDetach,
        exact: true,
      });
      await expect(detachDialog).toContainText(labels.inboxDetachDescription);
      await page.keyboard.press("Escape");
      await expect(inboxRow).toContainText(labels.inboxProcessed);
      await inboxRow
        .getByRole("button", { name: labels.inboxDetach, exact: true })
        .click();
      await detachDialog
        .getByRole("button", { name: labels.inboxDetachConfirm, exact: true })
        .click();
      await expect(detachDialog).not.toBeVisible();
      await expect(inboxRow).toContainText(labels.inboxPending);
      await page.reload();
      await expect(inboxRow).toContainText(labels.inboxPending);
      await inboxRow
        .getByRole("button", { name: labels.inboxAttach, exact: true })
        .click();
      await page
        .getByRole("dialog", { name: labels.inboxAttach, exact: true })
        .getByRole("button", { name: /Reviewed supplier snapshot/ })
        .click();
      await expect(inboxRow).toContainText(labels.inboxProcessed);
      await inboxRow
        .getByRole("link", { name: labels.inboxDocument, exact: true })
        .click();
      await expect(page).toHaveURL(processedDocumentUrl);
      await page.goto(`/#/companies/${alpha.id}/inbox`);
      await page.getByLabel(labels.inboxUpload, { exact: true }).setInputFiles({
        name: "synthetic-preview.pdf",
        mimeType: "application/pdf",
        buffer: syntheticPreviewPdf(),
      });
      await page
        .getByRole("row")
        .filter({ hasText: "synthetic-preview.pdf" })
        .getByRole("link", { name: labels.inboxCreate, exact: true })
        .click();
      const preview = page.getByRole("complementary", {
        name: labels.inboxOriginal,
        exact: true,
      });
      await expect(preview.getByRole("img")).toBeVisible();
      await expect(preview.getByRole("status")).not.toBeVisible();
      expect(
        await preview
          .getByRole("img")
          .evaluate((canvas) => (canvas as HTMLCanvasElement).width),
      ).toBeGreaterThan(100);
      expect(messages).toHaveLength(0);
      await page.goto(`/#/companies/${alpha.id}/inbox`);
      await inboxRow
        .getByRole("button", { name: labels.inboxOriginalEmail, exact: true })
        .click();
      const originalEmail = page.getByRole("dialog", {
        name: labels.inboxOriginalEmail,
        exact: true,
      });
      await expect(originalEmail).toContainText(
        labels.inboxOriginalEmailDescription,
      );
      await originalEmail
        .getByLabel(labels.quoteTo, { exact: true })
        .fill("accountant@example.test");
      const queueOriginal = originalEmail.getByRole("button", {
        name: labels.inboxOriginalQueue,
        exact: true,
      });
      await expect(queueOriginal).toBeDisabled();
      await originalEmail.getByRole("checkbox").check();
      await queueOriginal.click();
      await expect(
        originalEmail.getByText(labels.quoteQueuedNotice, { exact: true }),
      ).toBeVisible();
      await expect.poll(() => messages.length).toBe(1);
      expect(messages[0]).toContain("accountant@example.test");
      expect(messages[0]).toContain("synthetic.isdoc");
      await originalEmail
        .getByRole("button", {
          name: labels.inboxOriginalHistoryRefresh,
          exact: true,
        })
        .click();
      await expect(
        originalEmail.getByText(labels.quoteSent, { exact: true }),
      ).toBeVisible();
      await page.keyboard.press("Escape");
      await page.reload();
      await inboxRow
        .getByRole("button", { name: labels.inboxOriginalEmail, exact: true })
        .click();
      await expect(
        originalEmail.getByText(labels.quoteSent, { exact: true }),
      ).toBeVisible();
      expect(messages).toHaveLength(1);
    } catch (error) {
      await page.screenshot({
        path: `test-results/received-failure-${english ? "en" : "cs"}-${info.project.name}.png`,
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
