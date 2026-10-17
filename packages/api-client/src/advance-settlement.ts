import type { components } from "./advance-settlement.generated";
import {
  InvoiceCalculationFailure,
  parseInvoiceTotals,
  type InvoiceCalculation,
} from "./invoice";
import { getDraft } from "./draft";
export type AdvanceSettlement = components["schemas"]["AdvanceSettlement"];
export type SettlementConfigure = components["schemas"]["SettlementConfigure"];
export type SettlementCandidates =
  components["schemas"]["SettlementCandidates"];
async function request(
  b: string,
  c: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${b.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}${path}`,
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
  const v = await r.json().catch(() => null);
  if (!r.ok)
    throw new InvoiceCalculationFailure(
      r.status,
      v?.code ?? "unavailable",
      v?.fields ?? [],
    );
  return v;
}
export async function settlementCandidates(
  b: string,
  c: string,
  query: string,
  page: number,
  signal?: AbortSignal,
): Promise<SettlementCandidates> {
  const v = await request(
    b,
    c,
    `/advance-settlements?${new URLSearchParams({ query, page: String(page) })}`,
    undefined,
    signal,
  );
  if (
    !v ||
    !Array.isArray(v.items) ||
    ![v.total, v.page].every(Number.isSafeInteger) ||
    !v.items.every(
      (d: any) =>
        typeof d.id === "string" &&
        Number.isSafeInteger(d.revision) &&
        d.kind === "advance" &&
        typeof d.data?.invoice?.customer?.name === "string" &&
        typeof d.totals?.payable === "string",
    )
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return v;
}
export async function configureSettlement(
  b: string,
  c: string,
  cmd: SettlementConfigure,
) {
  const v = await request(b, c, "/advance-settlements", cmd);
  if (typeof v?.id !== "string")
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return getDraft(b, c, v.id);
}
export async function previewSettlement(
  b: string,
  c: string,
  id: string,
  calc: InvoiceCalculation,
  signal?: AbortSignal,
) {
  return parseInvoiceTotals(
    await request(
      b,
      c,
      `/invoices/${encodeURIComponent(id)}/advance-preview`,
      calc,
      signal,
    ),
  );
}
