import { isDocumentCurrency } from "./invoice";
import { parseSettlement } from "./payment";
import type { components } from "./invoice.generated";
import { InvoiceCalculationFailure, parseInvoiceTotals } from "./invoice";
export type InvoiceDraft = components["schemas"]["Draft"];
export type DraftData = components["schemas"]["DraftData"];
export type PartySnapshot = components["schemas"]["PartySnapshot"];
export type DraftPage = components["schemas"]["DraftPage"];
export type DraftSearch = {
  status?:
    | "all"
    | "draft"
    | "issued"
    | "outstanding"
    | "overdue"
    | "paid"
    | "partial"
    | "overpaid";
  query: string;
  page: number;
  sort: string;
  customer_id?: string;
  page_size?: number;
  date_field?: string;
  date_from?: string;
  date_to?: string;
  amount_min?: string;
  amount_max?: string;
};
async function request(
  base: string,
  company: string,
  path: string,
  method = "GET",
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/invoices${path}`,
    {
      method,
      credentials: "include",
      signal: signal ?? AbortSignal.timeout(15000),
      headers:
        body === undefined
          ? {}
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
    },
  );
  const v = await r.json().catch(() => null);
  if (!r.ok)
    throw new InvoiceCalculationFailure(
      r.status,
      v?.code ?? "unavailable",
      Array.isArray(v?.fields) ? v.fields : [],
    );
  return v;
}
function parse(v: any): InvoiceDraft {
  const party = (p: any) =>
    p &&
    [
      p.name,
      p.registration_number,
      p.vat_number,
      p.email,
      p.phone,
      p.address?.street,
      p.address?.city,
      p.address?.postal_code,
      p.address?.country,
    ].every((v) => typeof v === "string");
  const d = v?.data,
    c = d?.calculation;
  if (
    !v ||
    !(v.number === null || typeof v.number === "string") ||
    !(v.issued_at === null || typeof v.issued_at === "string") ||
    (v.number === null) !== (v.issued_at === null) ||
    (v.number !== null && v.totals === null) ||
    typeof v.id !== "string" ||
    typeof v.company_id !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    !party(d?.supplier) ||
    !(d.customer === null || party(d.customer)) ||
    !(d.contact_id === null || typeof d.contact_id === "string") ||
    ![
      d.note,
      d.order_number,
      d.language,
      d.payment?.method,
      d.payment?.bank_account,
      d.payment?.iban,
      d.payment?.variable_symbol,
    ].every((x) => typeof x === "string") ||
    ![d.issued_on, d.due_on, d.tax_on].every(
      (x) => x === null || typeof x === "string",
    ) ||
    !c ||
    !Array.isArray(c.lines) ||
    !c.lines.every((l: any) =>
      [
        l.id,
        l.description,
        l.quantity,
        l.unit,
        l.unit_price,
        l.discount_percent,
        l.vat_percent,
      ].every((x) => typeof x === "string"),
    ) ||
    !isDocumentCurrency(c.currency) ||
    !["of-czk-v1", "import-v1"].includes(c.calculation_version) ||
    !["non_payer", "vat_payer"].includes(c.tax_mode) ||
    !["net", "gross"].includes(c.price_mode) ||
    !["none", "whole_czk"].includes(c.rounding) ||
    typeof c.discount_percent !== "string" ||
    !(v.totals === null || typeof v.totals?.payable === "string")
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  if (v.totals !== null) parseInvoiceTotals(v.totals);
  if (v.settlement !== null) parseSettlement(v.settlement);
  return v;
}
export async function listDrafts(
  base: string,
  company: string,
  search: DraftSearch,
  signal?: AbortSignal,
): Promise<DraftPage> {
  const p = new URLSearchParams({
    query: search.query,
    status: search.status ?? "all",
    page: String(search.page),
    sort: search.sort,
    page_size: String(search.page_size ?? 25),
  });
  for (const field of [
    "date_field",
    "date_from",
    "date_to",
    "amount_min",
    "amount_max",
  ] as const) {
    if (search[field]) p.set(field, search[field]);
  }
  if (search.customer_id) p.set("customer_id", search.customer_id);
  const v = await request(base, company, `?${p}`, "GET", undefined, signal);
  if (
    !v ||
    !Array.isArray(v.items) ||
    ![v.total, v.page, v.page_size].every(Number.isSafeInteger)
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return { ...v, items: v.items.map(parse) };
}
export async function getDraft(
  base: string,
  company: string,
  id: string,
  signal?: AbortSignal,
) {
  return parse(
    await request(
      base,
      company,
      `/${encodeURIComponent(id)}`,
      "GET",
      undefined,
      signal,
    ),
  );
}
export async function createDraft(
  base: string,
  company: string,
  body: components["schemas"]["CreateDraft"],
) {
  return parse(await request(base, company, "", "POST", body));
}
export async function updateDraft(
  base: string,
  company: string,
  id: string,
  body: components["schemas"]["UpdateDraft"],
) {
  return parse(
    await request(base, company, `/${encodeURIComponent(id)}`, "PUT", body),
  );
}

export async function issueInvoice(
  base: string,
  company: string,
  id: string,
  body: components["schemas"]["IssueInvoice"],
) {
  return parse(
    await request(
      base,
      company,
      `/${encodeURIComponent(id)}/issue`,
      "POST",
      body,
    ),
  );
}
export async function invoicePdf(base: string, company: string, id: string) {
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/invoices/${encodeURIComponent(id)}/pdf`,
    { credentials: "include", signal: AbortSignal.timeout(60000) },
  );
  if (!response.ok)
    throw new InvoiceCalculationFailure(response.status, "pdf_failed");
  return response.blob();
}
