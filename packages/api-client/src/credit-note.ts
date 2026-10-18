import { isDocumentCurrency } from "./invoice";
import type { components } from "./credit-note.generated";
import { SalesFailure } from "./sales";
export type CreditNote = components["schemas"]["CreditDocument"];
export type CreditData = components["schemas"]["CreditData"];
export type CreditCommand = components["schemas"]["CreditCommand"];
export type CreditPage = components["schemas"]["CreditPage"];
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

function document(v: unknown): CreditNote {
  const d = v as CreditNote;
  const money = (v: unknown) =>
    typeof v === "string" && /^-?\d+\.\d{2}$/.test(v);
  if (
    !d ||
    ![d.id, d.company_id, d.invoice_id, d.invoice_number].every(
      (v) => typeof v === "string",
    ) ||
    !Number.isSafeInteger(d.revision) ||
    !["draft", "issued", "cancelled"].includes(d.status) ||
    !d.source?.data?.supplier ||
    !Array.isArray(d.data?.lines) ||
    !isDocumentCurrency(d.totals?.currency) ||
    !Array.isArray(d.totals?.tax_summary) ||
    !Array.isArray(d.events) ||
    ![d.totals?.payable, d.remaining, d.offset, d.refunded].every(money)
  )
    throw new SalesFailure(502, "invalid_response");
  return d;
}
export async function createCredit(
  b: string,
  c: string,
  invoice: string,
  key: string,
) {
  return document(
    await (
      await request(b, c, "/credit-notes", {
        invoice_id: invoice,
        request_id: key,
      })
    ).json(),
  );
}
export async function creditCommand(
  b: string,
  c: string,
  id: string,
  cmd: CreditCommand,
) {
  return document(
    await (
      await request(b, c, `/credit-notes/${encodeURIComponent(id)}`, cmd)
    ).json(),
  );
}
export async function getCredit(
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
        `/credit-notes/${encodeURIComponent(id)}`,
        undefined,
        signal,
      )
    ).json(),
  );
}
export async function listCredits(
  b: string,
  c: string,
  query: string,
  page: number,
  invoice?: string,
  signal?: AbortSignal,
): Promise<CreditPage> {
  const q = new URLSearchParams({ query, page: String(page) });
  if (invoice) q.set("invoice_id", invoice);
  const v = await (
    await request(b, c, `/credit-notes?${q}`, undefined, signal)
  ).json();
  if (
    !v ||
    !Array.isArray(v.items) ||
    ![v.page, v.total].every(Number.isSafeInteger)
  )
    throw new SalesFailure(502, "invalid_response");
  return { ...v, items: v.items.map(document) };
}
export async function creditPdf(b: string, c: string, id: string) {
  return (
    await request(b, c, `/credit-notes/${encodeURIComponent(id)}/pdf`)
  ).blob();
}
