import type { Page } from "@playwright/test";

// Synthetic, read-only UI fixtures. Every API call is intercepted, never forwarded.
export async function mockRedesignWorkspace(
  page: Page,
  options: {
    language?: "cs" | "en";
    dark?: boolean;
    failReport?: boolean;
    viewer?: boolean;
  } = {},
) {
  await page.addInitScript(
    ({ language }) => localStorage.setItem("open-faktura.language", language),
    { language: options.language ?? "cs" },
  );
  const party = {
    name: "Ukázková firma",
    registration_number: "",
    vat_number: "",
    email: "",
    phone: "",
    address: {
      street: "Ukázková 1",
      city: "Praha",
      postal_code: "12000",
      country: "CZ",
    },
  };
  const company = {
    id: "design",
    revision: 1,
    step: 0,
    completed: true,
    role: options.viewer ? "viewer" : "owner",
    settings: {
      name: party.name,
      entity_type: "company",
      registration_number: "",
      vat_number: "",
      street: "Ukázková 1",
      city: "Praha",
      postal_code: "12000",
      country: "CZ",
      email: "",
      phone: "",
      tax_mode: "vat_payer",
      tax_effective_from: "",
      currency: "CZK",
      timezone: "Europe/Prague",
      payment_method: "bank_transfer",
      iban: "",
      number_prefix: "FV",
      maturity_days: 14,
      number_year: 2026,
      next_number: 43,
    },
    missing_fields: [],
    number_preview: "FV2026000043",
    can_issue: true,
    tax_history: [],
  };
  const amounts = { net: "20000.00", vat: "4200.00", gross: "24200.00" };
  const totals = {
    calculation_version: "of-czk-v1",
    currency: "CZK",
    amounts,
    rounding: "0.00",
    payable: "24200.00",
    lines: [{ id: "line", amounts }],
    tax_summary: [{ tax_mode: "vat_payer", vat_percent: "21", amounts }],
  };
  const invoice = (
    index: number,
    customer: string,
    status: "paid" | "unpaid",
    overdue = false,
  ) => ({
    id: `invoice-${index}`,
    company_id: company.id,
    revision: 1,
    number: `2026-0${42 - index}`,
    issued_at: "2026-10-16T10:00:00Z",
    settlement: {
      paid: status === "paid" ? "24200.00" : "0.00",
      outstanding: status === "paid" ? "0.00" : "24200.00",
      overpayment: "0.00",
      status,
      overdue,
      revision: 1,
      today: "2026-10-17",
    },
    totals,
    data: {
      supplier: party,
      customer: { ...party, name: customer },
      contact_id: null,
      note: "Děkujeme za spolupráci.",
      order_number: "",
      language: options.language ?? "cs",
      issued_on: "2026-10-16",
      due_on: overdue ? "2026-10-15" : "2026-10-30",
      tax_on: "2026-10-16",
      payment: {
        method: "bank_transfer",
        bank_account: "",
        iban: "",
        variable_symbol: "",
      },
      calculation: {
        calculation_version: "of-czk-v1",
        tax_mode: "vat_payer",
        currency: "CZK",
        price_mode: "net",
        rounding: "none",
        discount_percent: "0",
        lines: [
          {
            id: "line",
            description: "Grafické práce",
            quantity: "20",
            unit: "hod",
            unit_price: "1000",
            discount_percent: "0",
            vat_percent: "21",
          },
        ],
      },
    },
  });
  const items = [
    invoice(0, "Studio Lípa", "unpaid"),
    invoice(1, "Ateliér Sever", "paid"),
    invoice(2, "Modrý dům", "unpaid", true),
    invoice(3, "Dílna Most", "unpaid"),
    invoice(4, "Papír & plán", "paid"),
    {
      ...invoice(5, "Nový prostor", "unpaid"),
      id: "draft",
      number: null,
      issued_at: null,
      settlement: null,
    },
  ];
  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    if (path === "/api/auth/setup")
      return route.fulfill({ json: { setup_required: false } });
    if (path === "/api/auth/session")
      return route.fulfill({
        json: {
          id: "design-user",
          name: "Jan Moudrý",
          email: "design@example.test",
          installation_admin: false,
        },
      });
    if (path === "/api/auth/preferences")
      return route.fulfill({
        json: { revision: 1, theme: options.dark ? "dark" : "light" },
      });
    if (path === "/api/companies") return route.fulfill({ json: [company] });
    if (path === "/api/companies/design")
      return route.fulfill({ json: company });
    if (path.endsWith("/overview"))
      return route.fulfill({
        json: {
          today: "2026-10-17",
          currency: "CZK",
          issued_count: 18,
          draft_count: 3,
          outstanding_count: 6,
          overdue_count: 1,
          outstanding: "86400.00",
          overdue: "12800.00",
          received: "162200.00",
          overpayment: "0.00",
        },
      });
    if (path.endsWith("/reports/period")) {
      if (options.failReport)
        return route.fulfill({ status: 503, json: { code: "unavailable" } });
      return route.fulfill({
        json: {
          sales: "248600.00",
          purchases: "112400.00",
          currency: "CZK",
          total: 27,
          page: 1,
          items: [],
          comparison: null,
          months: [
            { month: "2026-08", sales: "68000.00", purchases: "32400.00" },
            { month: "2026-09", sales: "88600.00", purchases: "36000.00" },
            { month: "2026-10", sales: "92000.00", purchases: "44000.00" },
          ],
        },
      });
    }
    if (path.endsWith("/invoices"))
      return route.fulfill({
        json: { items, total: 42, page: 1, page_size: 10 },
      });
    if (path.endsWith("/invoices/draft"))
      return route.fulfill({ json: items[5] });
    if (path.endsWith("/invoices/calculate"))
      return route.fulfill({ json: totals });
    if (path.endsWith("/sales"))
      return route.fulfill({
        json: { items: [], total: 0, page: 1, page_size: 25 },
      });
    if (path.endsWith("/contacts"))
      return route.fulfill({
        json: { items: [], total: 0, page: 1, page_size: 25 },
      });
    return route.fulfill({ status: 503, json: { code: "unavailable" } });
  });
}
