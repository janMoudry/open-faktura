import type { components } from "./invoice-share.generated";
import { SalesFailure } from "./sales";
import type { ShareLink, ShareLinks, ShareViews } from "./sales";
async function request(
  base: string,
  c: string,
  path: string,
  method = "GET",
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/invoices${path}`,
    {
      method,
      credentials: "include",
      headers:
        method === "GET"
          ? undefined
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: signal ?? AbortSignal.timeout(30000),
    },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable");
  }
  const v = await r.json();
  if (
    v !== null &&
    (!v ||
      typeof v !== "object" ||
      ("items" in v
        ? !Array.isArray(v.items) || !Number.isSafeInteger(v.total)
        : typeof v.id !== "string" || typeof v.url !== "string"))
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function listInvoiceShares(
  base: string,
  c: string,
  d: string,
  page = 1,
  signal?: AbortSignal,
): Promise<ShareLinks> {
  return request(
    base,
    c,
    `/${encodeURIComponent(d)}/shares?page=${page}`,
    "GET",
    undefined,
    signal,
  );
}
export async function createInvoiceShare(
  base: string,
  c: string,
  d: string,
  cmd: components["schemas"]["CreateShare"],
): Promise<ShareLink> {
  return request(base, c, `/${encodeURIComponent(d)}/shares`, "POST", cmd);
}
export async function revokeInvoiceShare(
  base: string,
  c: string,
  d: string,
  id: string,
): Promise<void> {
  return request(
    base,
    c,
    `/${encodeURIComponent(d)}/shares/${encodeURIComponent(id)}/revoke`,
    "POST",
    {},
  );
}
export async function invoiceShareViews(
  base: string,
  c: string,
  d: string,
  id: string,
  page = 1,
  signal?: AbortSignal,
): Promise<ShareViews> {
  return request(
    base,
    c,
    `/${encodeURIComponent(d)}/shares/${encodeURIComponent(id)}/views?page=${page}`,
    "GET",
    undefined,
    signal,
  );
}
