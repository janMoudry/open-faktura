import { test, expect } from "@playwright/test";
import { mockRedesignWorkspace } from "./redesign-fixture";
for (const language of ["cs", "en"] as const) {
  test(`company administration tags and brand save and reload ${language}`, async ({
    page,
  }) => {
    await mockRedesignWorkspace(page, { language });
    const text = (cs: string, en: string) => (language === "cs" ? cs : en);
    let tags: {
      id: string;
      revision: number;
      name: string;
      usage_count: number;
    }[] = [];
    let revision = 1;
    let members = [
      {
        user_id: "1",
        email: "owner@example.test",
        name: "Synthetic owner",
        role: "owner",
      },
    ];
    let style = {
      revision: 0,
      data: {
        logo_base64: null as string | null,
        signature_base64: null as string | null,
        footer_cs: "",
        footer_en: "",
      },
    };
    await page.route("**/api/companies/design/tags", async (route) => {
      if (route.request().method() === "POST") {
        const body = route.request().postDataJSON();
        if (body.action === "create")
          tags.push({
            id: "tag-1",
            revision: 1,
            name: body.name,
            usage_count: 0,
          });
        if (body.action === "rename")
          tags = tags.map((t) => ({
            ...t,
            name: body.name,
            revision: t.revision + 1,
          }));
        if (body.action === "delete") tags = [];
      }
      await route.fulfill({ json: tags });
    });
    await page.route(
      "**/api/companies/design/administration",
      async (route) => {
        if (route.request().method() === "POST") {
          const b = route.request().postDataJSON();
          if (b.action === "add_member") {
            members.push({
              user_id: "2",
              email: b.email,
              name: "",
              role: b.role,
            });
            revision++;
          }
        }
        await route.fulfill({
          json: { company_id: "design", revision, deleted: false, members },
        });
      },
    );
    await page.route(
      "**/api/companies/design/document-style",
      async (route) => {
        if (route.request().method() === "POST") {
          const b = route.request().postDataJSON();
          style = { revision: style.revision + 1, data: b.data };
        }
        await route.fulfill({ json: style });
      },
    );
    await page.goto("/#/companies/design");
    await page
      .getByLabel(text("Název štítku", "Tag name"), { exact: true })
      .fill("Synthetic tag");
    await page
      .getByRole("button", {
        name: text("Vytvořit štítek", "Create tag"),
        exact: true,
      })
      .click();
    await expect(
      page.getByText("Synthetic tag (0)", { exact: true }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: text("Přejmenovat", "Rename"), exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await dialog
      .getByLabel(text("Název štítku", "Tag name"), { exact: true })
      .fill("Renamed tag");
    await dialog
      .getByRole("button", { name: text("Přejmenovat", "Rename"), exact: true })
      .click();
    await expect(dialog).not.toBeVisible();
    await expect(
      page.getByText("Renamed tag (0)", { exact: true }),
    ).toBeVisible();
    const admin = page.locator("section").filter({
      has: page.getByRole("heading", {
        name: text("Přístupy a vlastnictví", "Access and ownership"),
        exact: true,
      }),
    });
    await admin
      .getByLabel(text("E-mail", "Email"), { exact: true })
      .fill("colleague@example.test");
    await admin
      .getByRole("button", {
        name: text("Přidat existujícího uživatele", "Add existing user"),
        exact: true,
      })
      .click();
    await expect(admin.getByText(/colleague@example.test/)).toBeVisible();
    await admin
      .getByRole("button", {
        name: text("Převést vlastnictví", "Transfer ownership"),
        exact: true,
      })
      .click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: text("Zrušit", "Cancel"), exact: true })
      .click();
    await page
      .getByLabel(
        text("Text před položkami (čeština)", "Text before items (Czech)"),
        { exact: true },
      )
      .fill("Synthetic Czech introduction\nSecond line");
    await page
      .getByLabel(
        text("Text před položkami (angličtina)", "Text before items (English)"),
        { exact: true },
      )
      .fill("Synthetic English introduction");
    await page
      .getByLabel(text("Zápatí v češtině", "Czech footer"), { exact: true })
      .fill("Synthetic Czech footer");
    await page
      .getByLabel(text("Zápatí v angličtině", "English footer"), {
        exact: true,
      })
      .fill("Synthetic English footer");
    await page
      .getByLabel(text("Logo", "Logo"), { exact: true })
      .setInputFiles("tests/fixtures/document-logo.png");
    await expect(
      page.getByRole("img", { name: text("Logo", "Logo"), exact: true }),
    ).toBeVisible();
    await page
      .getByRole("checkbox", {
        name: text(
          "Dodací listy standardně tisknout s cenami",
          "Include prices on delivery notes by default",
        ),
        exact: true,
      })
      .check();
    await page
      .getByRole("button", {
        name: text("Uložit vzhled", "Save appearance"),
        exact: true,
      })
      .click();
    await expect(
      page.getByRole("button", {
        name: text("Uložit vzhled", "Save appearance"),
        exact: true,
      }),
    ).toBeDisabled();
    await page.reload();
    await expect(
      page.getByLabel(
        text("Text před položkami (čeština)", "Text before items (Czech)"),
        { exact: true },
      ),
    ).toHaveValue("Synthetic Czech introduction\nSecond line");
    await expect(
      page.getByLabel(
        text("Text před položkami (angličtina)", "Text before items (English)"),
        { exact: true },
      ),
    ).toHaveValue("Synthetic English introduction");
    await expect(
      page.getByLabel(text("Zápatí v angličtině", "English footer"), {
        exact: true,
      }),
    ).toHaveValue("Synthetic English footer");
    await expect(
      page.getByRole("checkbox", {
        name: text(
          "Dodací listy standardně tisknout s cenami",
          "Include prices on delivery notes by default",
        ),
        exact: true,
      }),
    ).toBeChecked();

    await expect(
      page.getByText("Renamed tag (0)", { exact: true }),
    ).toBeVisible();
    await expect(page.getByText(/colleague@example.test/)).toBeVisible();
    await page
      .getByRole("button", { name: text("Odstranit", "Delete"), exact: true })
      .click();
    await expect(page.getByRole("dialog")).toContainText(
      text(
        "Odstranit štítek a jeho přiřazení",
        "Delete this tag and its assignments",
      ),
    );
    await page
      .getByRole("dialog")
      .getByRole("button", { name: text("Odstranit", "Delete"), exact: true })
      .click();
    await expect(
      page.getByText("Renamed tag (0)", { exact: true }),
    ).toHaveCount(0);
  });
  test(`company metadata viewer permissions ${language}`, async ({ page }) => {
    await mockRedesignWorkspace(page, { language, viewer: true });
    const text = (cs: string, en: string) => (language === "cs" ? cs : en);
    await page.route("**/api/companies/design/tags", (r) =>
      r.fulfill({
        json: [{ id: "1", revision: 1, name: "Read-only tag", usage_count: 0 }],
      }),
    );
    await page.route("**/api/companies/design/administration", (r) =>
      r.fulfill({
        json: {
          company_id: "design",
          revision: 1,
          deleted: false,
          members: [
            {
              user_id: "1",
              email: "owner@example.test",
              name: "",
              role: "owner",
            },
          ],
        },
      }),
    );
    await page.route("**/api/companies/design/document-style", (r) =>
      r.fulfill({
        json: {
          revision: 0,
          data: {
            logo_base64: null,
            signature_base64: null,
            footer_cs: "",
            footer_en: "",
          },
        },
      }),
    );
    await page.goto("/#/companies/design");
    await expect(
      page.getByText("Read-only tag (0)", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", {
        name: text("Vytvořit štítek", "Create tag"),
        exact: true,
      }),
    ).toHaveCount(0);
    await expect(
      page.getByLabel(text("Zápatí v angličtině", "English footer"), {
        exact: true,
      }),
    ).toBeDisabled();
    await expect(
      page.getByRole("button", {
        name: text("Uložit vzhled", "Save appearance"),
        exact: true,
      }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("button", {
        name: text("Opustit firmu", "Leave company"),
        exact: true,
      }),
    ).toBeVisible();
  });
}

for (const language of ["cs", "en"] as const) {
  test(`tag filters reach register API and persist in URL ${language}`, async ({
    page,
  }) => {
    await mockRedesignWorkspace(page, { language });
    await page.route("**/api/companies/design/tags", (r) =>
      r.fulfill({
        json: [
          {
            id: "1",
            revision: 1,
            name: "Synthetic project filter",
            usage_count: 4,
          },
        ],
      }),
    );
    const requests = new Map<string, string>();
    for (const endpoint of [
      "contacts",
      "invoices",
      "sales",
      "received-invoices",
    ]) {
      await page.route(`**/api/companies/design/${endpoint}?*`, (r) => {
        const url = new URL(r.request().url());
        requests.set(endpoint, url.searchParams.get("tag_id") ?? "");
        return r.fulfill({
          json: { items: [], total: 0, page: 1, page_size: 25 },
        });
      });
    }
    for (const [path, endpoint] of [
      ["contacts", "contacts"],
      ["invoices", "invoices"],
      ["quotes", "sales"],
      ["received-invoices", "received-invoices"],
    ]) {
      await page.goto(`/#/companies/design/${path}`);
      const selector = page.getByRole("button", {
        name: new RegExp(
          language === "cs" ? "Filtrovat podle štítku" : "Filter by tag",
        ),
      });
      await selector.click();
      await page
        .getByRole("option", { name: "Synthetic project filter", exact: true })
        .click();
      await expect(page).toHaveURL(/tag=1/);
      await expect.poll(() => requests.get(endpoint)).toBe("1");
      await page.reload();
      await expect(selector).toContainText("Synthetic project filter");
      await expect.poll(() => requests.get(endpoint)).toBe("1");
    }
  });
}
