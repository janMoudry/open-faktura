import type { components } from "./received.generated";
import { SalesFailure } from "./sales";
export type ReceivedInvoice = components["schemas"]["ReceivedDocument"];
export type ReceivedData = components["schemas"]["ReceivedData"];
export type ReceivedCommand = components["schemas"]["ReceivedCommand"];
export type ReceivedPage = components["schemas"]["ReceivedPage"];
export type ReceivedTotals = components["schemas"]["ReceivedTotals"];
async function request(
  b: string,
  c: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${b.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/received-invoices${path}`,
    {
      method: body === undefined ? "GET" : "POST",
      credentials: "include",
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
  return r.json();
}
function document(d: ReceivedInvoice): ReceivedInvoice {
  if (
    !d ||
    typeof d.id !== "string" ||
    typeof d.company_id !== "string" ||
    typeof d.number !== "string" ||
    !Number.isSafeInteger(d.revision) ||
    !d.data?.supplier ||
    !Array.isArray(d.data.lines) ||
    !Array.isArray(d.payments) ||
    typeof d.cancelled !== "boolean" ||
    ![d.totals?.total, d.remaining, d.paid].every(
      (x) => typeof x === "string" && /^\d+\.\d{2}$/.test(x),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return d;
}
export async function listReceived(
  b: string,
  c: string,
  query: string,
  status: string,
  page: number,
  signal?: AbortSignal,
  kind: "invoice" | "receipt" = "invoice",
): Promise<ReceivedPage> {
  const v = await request(
    b,
    c,
    `?${new URLSearchParams({ query, status, page: String(page), kind })}`,
    undefined,
    signal,
  );
  if (!Array.isArray(v?.items) || !Number.isSafeInteger(v.total))
    throw new SalesFailure(502, "invalid_response");
  return { ...v, items: v.items.map(document) };
}
export async function getReceived(
  b: string,
  c: string,
  id: string,
  signal?: AbortSignal,
) {
  return document(
    await request(b, c, `/${encodeURIComponent(id)}`, undefined, signal),
  );
}
export async function createReceived(
  b: string,
  c: string,
  data: ReceivedData,
  request_id: string,
) {
  return document(await request(b, c, "", { data, request_id }));
}
export async function commandReceived(
  b: string,
  c: string,
  id: string,
  cmd: ReceivedCommand,
) {
  return document(await request(b, c, `/${encodeURIComponent(id)}`, cmd));
}
export async function previewReceived(
  b: string,
  c: string,
  data: ReceivedData,
  signal?: AbortSignal,
): Promise<ReceivedTotals> {
  const v = await request(b, c, "/preview", data, signal);
  if (
    !v ||
    ![v.net, v.vat, v.total].every(
      (x) => typeof x === "string" && /^\d+\.\d{2}$/.test(x),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}

export type ExpensePage = components["schemas"]["ExpensePage"];
export type ExpenseOverview = components["schemas"]["ExpenseOverview"];
export async function getExpensePayments(
  b: string,
  c: string,
  params: Record<string, string>,
  signal?: AbortSignal,
): Promise<ExpensePage> {
  const v = await request(
    b,
    c,
    `/expense-payments?${new URLSearchParams(params)}`,
    undefined,
    signal,
  );
  if (
    !Array.isArray(v?.items) ||
    !Number.isSafeInteger(v.total) ||
    !Number.isSafeInteger(v.page) ||
    v.items.some(
      (x: ExpensePage["items"][number]) =>
        !x ||
        typeof x.document_id !== "string" ||
        !["invoice", "receipt"].includes(x.kind) ||
        typeof x.payment?.amount !== "string" ||
        !/^\d+\.\d{2}$/.test(x.payment.amount),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function getExpenseOverview(
  b: string,
  c: string,
  from: string,
  to: string,
  signal?: AbortSignal,
): Promise<ExpenseOverview> {
  const v = await request(
    b,
    c,
    `/expense-overview?${new URLSearchParams({ from, to })}`,
    undefined,
    signal,
  );
  if (
    !v ||
    !["purchases", "expenses", "outstanding", "overdue"].every(
      (k) => typeof v[k] === "string" && /^\d+\.\d{2}$/.test(v[k]),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
