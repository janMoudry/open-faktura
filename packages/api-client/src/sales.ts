import type { components } from "./sales.generated";
export type SalesDocument = components["schemas"]["SalesDocument"];
export type SalesData = components["schemas"]["SalesData"];
export type SalesPage = components["schemas"]["SalesPage"];
export type SalesKind = "template" | "quote";
export class SalesFailure extends Error {
  constructor(
    public status: number,
    public code: string,
    public fields: string[] = [],
  ) {
    super(code);
  }
}
async function request(
  base: string,
  company: string,
  path: string,
  method = "GET",
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/sales${path}`,
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
    throw new SalesFailure(
      r.status,
      v?.code ?? "unavailable",
      Array.isArray(v?.fields) ? v.fields : [],
    );
  return v;
}
function document(v: unknown): SalesDocument {
  const d = v as SalesDocument;
  if (
    !d ||
    typeof d.id !== "string" ||
    typeof d.company_id !== "string" ||
    !Number.isSafeInteger(d.revision) ||
    !["template", "quote"].includes(d.kind) ||
    typeof d.archived !== "boolean" ||
    !d.data ||
    typeof d.data.name !== "string" ||
    !d.data.invoice?.calculation?.lines
  )
    throw new SalesFailure(502, "invalid_response");
  return d;
}
export async function listSales(
  base: string,
  company: string,
  search: {
    kind: SalesKind;
    query?: string;
    status?: string;
    archived?: boolean;
    page?: number;
    page_size?: number;
  },
  signal?: AbortSignal,
): Promise<SalesPage> {
  const p = new URLSearchParams(
    Object.entries(search).map(([k, v]) => [k, String(v)]),
  );
  const v = await request(base, company, `?${p}`, "GET", undefined, signal);
  if (
    !v ||
    !Array.isArray(v.items) ||
    ![v.total, v.page, v.page_size].every(Number.isSafeInteger)
  )
    throw new SalesFailure(502, "invalid_response");
  return { ...v, items: v.items.map(document) };
}
export async function getSales(
  base: string,
  c: string,
  id: string,
  signal?: AbortSignal,
) {
  return document(
    await request(
      base,
      c,
      `/${encodeURIComponent(id)}`,
      "GET",
      undefined,
      signal,
    ),
  );
}
export async function createSales(
  base: string,
  c: string,
  body: components["schemas"]["CreateSales"],
) {
  return document(await request(base, c, "", "POST", body));
}
export async function updateSales(
  base: string,
  c: string,
  id: string,
  body: components["schemas"]["UpdateSales"],
) {
  return document(
    await request(base, c, `/${encodeURIComponent(id)}`, "PUT", body),
  );
}
export async function convertSales(
  base: string,
  c: string,
  id: string,
  body: components["schemas"]["ConvertSales"],
): Promise<components["schemas"]["Draft"]> {
  const v = await request(
    base,
    c,
    `/${encodeURIComponent(id)}/convert`,
    "POST",
    body,
  );
  if (
    !v ||
    typeof v.id !== "string" ||
    typeof v.company_id !== "string" ||
    !v.data
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}

export type QuoteDelivery = components["schemas"]["Delivery"];
export type DeliveryHistory = components["schemas"]["DeliveryHistory"];
export type SendQuote = components["schemas"]["SendQuote"];
export async function quoteHistory(
  base: string,
  c: string,
  id: string,
  page = 1,
  signal?: AbortSignal,
): Promise<DeliveryHistory> {
  return request(
    base,
    c,
    `/${encodeURIComponent(id)}/deliveries?page=${page}`,
    "GET",
    undefined,
    signal,
  );
}
export async function sendQuote(
  base: string,
  c: string,
  id: string,
  command: SendQuote,
): Promise<QuoteDelivery> {
  return request(
    base,
    c,
    `/${encodeURIComponent(id)}/deliveries`,
    "POST",
    command,
  );
}
export async function quotePdf(
  base: string,
  c: string,
  id: string,
  delivery?: string,
): Promise<Blob> {
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/sales/${encodeURIComponent(id)}${delivery ? `/deliveries/${encodeURIComponent(delivery)}` : ""}/pdf`,
    { credentials: "include", signal: AbortSignal.timeout(30000) },
  );
  if (!response.ok) {
    const v = await response.json().catch(() => null);
    throw new SalesFailure(
      response.status,
      v?.code ?? "unavailable",
      v?.fields ?? [],
    );
  }
  return response.blob();
}

export type ShareLink = components["schemas"]["ShareLink"];
export type ShareLinks = components["schemas"]["ShareLinks"];
export type ShareViews = components["schemas"]["ShareViews"];
export async function listQuoteShares(
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
export async function createQuoteShare(
  base: string,
  c: string,
  d: string,
  cmd: components["schemas"]["CreateShare"],
): Promise<ShareLink> {
  return request(base, c, `/${encodeURIComponent(d)}/shares`, "POST", cmd);
}
export async function revokeQuoteShare(
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
export async function quoteShareViews(
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
