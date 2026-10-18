import type { components } from "./public-invoice.generated";
import { SalesFailure } from "./sales";
export type PublicInvoice = components["schemas"]["PublicInvoice"];
async function request(
  base: string,
  action: string,
  body: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/public/invoices/${action}`,
    {
      method: "POST",
      credentials: "omit",
      referrerPolicy: "no-referrer",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      signal: signal ?? AbortSignal.timeout(30000),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "public_invoice_unavailable");
  return r;
}
export async function getPublicInvoice(
  base: string,
  token: string,
  signal?: AbortSignal,
): Promise<PublicInvoice> {
  const r = await request(base, "resolve", { token }, signal);
  const v = await r.json();
  if (
    !v ||
    typeof v.number !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    !v.invoice?.supplier ||
    !v.invoice?.customer ||
    !v.totals?.payable
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function publicInvoicePdf(
  base: string,
  token: string,
  signal?: AbortSignal,
): Promise<Blob> {
  return (await request(base, "pdf", { token }, signal)).blob();
}
export async function recordInvoiceView(
  base: string,
  token: string,
  visit_id: string,
  revision: number,
  signal?: AbortSignal,
): Promise<void> {
  await request(base, "view", { token, visit_id, revision }, signal);
}
