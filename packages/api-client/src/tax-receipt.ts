import type { components } from "./tax-receipt.generated";
import { SalesFailure } from "./sales";
export type TaxReceipt = components["schemas"]["TaxDocument"];
export type TaxReceiptPage = components["schemas"]["TaxPage"];
export type IssueTaxReceipt = components["schemas"]["TaxIssue"];
async function request(
  base: string,
  c: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}${path}`,
    {
      credentials: "include",
      method: body === undefined ? "GET" : "POST",
      headers:
        body === undefined
          ? undefined
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: signal ?? AbortSignal.timeout(30000),
    },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  return r;
}
function document(v: unknown): TaxReceipt {
  const d = v as TaxReceipt;
  if (
    !d ||
    ![
      d.id,
      d.number,
      d.company_id,
      d.advance_id,
      d.payment_id,
      d.advance_number,
    ].every((x) => typeof x === "string") ||
    !d.data?.supplier ||
    !d.data?.customer ||
    !Array.isArray(d.data?.calculation?.lines) ||
    !Array.isArray(d.totals?.tax_summary) ||
    typeof d.totals?.payable !== "string"
  )
    throw new SalesFailure(502, "invalid_response");
  return d;
}
export async function previewTaxReceipt(
  b: string,
  c: string,
  d: string,
  p: string,
) {
  return document(
    await (
      await request(
        b,
        c,
        `/advances/${encodeURIComponent(d)}/payments/${encodeURIComponent(p)}/tax-preview`,
      )
    ).json(),
  );
}
export async function issueTaxReceipt(
  b: string,
  c: string,
  body: IssueTaxReceipt,
) {
  return document(await (await request(b, c, "/tax-receipts", body)).json());
}
export async function getTaxReceipt(
  b: string,
  c: string,
  id: string,
  signal?: AbortSignal,
) {
  return document(
    await (
      await request(
        b,
        c,
        `/tax-receipts/${encodeURIComponent(id)}`,
        undefined,
        signal,
      )
    ).json(),
  );
}
export async function taxReceiptPdf(b: string, c: string, id: string) {
  return (
    await request(b, c, `/tax-receipts/${encodeURIComponent(id)}/pdf`)
  ).blob();
}
export async function listTaxReceipts(
  b: string,
  c: string,
  query: string,
  page: number,
  pageSize: number,
  signal?: AbortSignal,
): Promise<TaxReceiptPage> {
  const v = await (
    await request(
      b,
      c,
      `/tax-receipts?${new URLSearchParams({ query, page: String(page), page_size: String(pageSize) })}`,
      undefined,
      signal,
    )
  ).json();
  if (
    !v ||
    !Array.isArray(v.items) ||
    ![v.total, v.page, v.page_size].every(Number.isSafeInteger)
  )
    throw new SalesFailure(502, "invalid_response");
  return { ...v, items: v.items.map(document) };
}
