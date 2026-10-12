import type { components } from "./invoice.generated";
import { InvoiceCalculationFailure } from "./invoice";
export type Settlement = components["schemas"]["Settlement"];
export type Payment = components["schemas"]["Payment"];
export type Payments = components["schemas"]["Payments"];
export type PaymentInput = components["schemas"]["PaymentInput"];
export type Overview = components["schemas"]["Overview"];
const money = (v: unknown) => typeof v === "string" && /^\d+\.\d{2}$/.test(v);
export function parseSettlement(v: any): Settlement {
  if (
    !v ||
    ![v.paid, v.outstanding, v.overpayment].every(money) ||
    !["unpaid", "partial", "paid", "overpaid"].includes(v.status) ||
    typeof v.overdue !== "boolean" ||
    !Number.isSafeInteger(v.revision) ||
    v.revision < 0 ||
    typeof v.today !== "string"
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return v;
}
function parse(v: any): Payments {
  if (
    !v ||
    !Array.isArray(v.items) ||
    !v.items.every(
      (p: any) =>
        p &&
        money(p.amount) &&
        ["cash", "bank_transfer"].includes(p.method) &&
        [p.id, p.paid_on, p.method, p.note, p.actor_id, p.created_at].every(
          (x) => typeof x === "string",
        ) &&
        [p.reversed_at, p.reason, p.reversed_by, p.replacement_id].every(
          (x) => x === null || typeof x === "string",
        ),
    )
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return { ...v, summary: parseSettlement(v.summary) };
}
async function request(
  base: string,
  company: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}${path}`,
    {
      method: body === undefined ? "GET" : "POST",
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
      v?.fields ?? [],
    );
  return v;
}
export async function getPayments(
  base: string,
  company: string,
  id: string,
  signal?: AbortSignal,
) {
  return parse(
    await request(
      base,
      company,
      `/invoices/${encodeURIComponent(id)}/payments`,
      undefined,
      signal,
    ),
  );
}
export async function registerPayment(
  base: string,
  company: string,
  id: string,
  body: components["schemas"]["RegisterPayment"],
) {
  return parse(
    await request(
      base,
      company,
      `/invoices/${encodeURIComponent(id)}/payments`,
      body,
    ),
  );
}
export async function reversePayment(
  base: string,
  company: string,
  id: string,
  payment: string,
  body: components["schemas"]["ReversePayment"],
) {
  return parse(
    await request(
      base,
      company,
      `/invoices/${encodeURIComponent(id)}/payments/${encodeURIComponent(payment)}/reverse`,
      body,
    ),
  );
}
export async function getOverview(
  base: string,
  company: string,
  signal?: AbortSignal,
): Promise<Overview> {
  const v = await request(base, company, "/overview", undefined, signal);
  if (
    !v ||
    ![
      v.issued_count,
      v.draft_count,
      v.outstanding_count,
      v.overdue_count,
    ].every((x) => Number.isSafeInteger(x) && x >= 0) ||
    ![v.outstanding, v.overdue, v.received, v.overpayment].every(money) ||
    v.currency !== "CZK" ||
    typeof v.today !== "string"
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return v;
}
