import type { components } from "./stock.generated";
import { SalesFailure } from "./sales";
export type StockCard = components["schemas"]["StockCard"];
export type StockMovement = components["schemas"]["StockMovement"];
export type StockPage = components["schemas"]["StockPage"];
export type StockCommand = components["schemas"]["StockCommand"];
async function request(
  b: string,
  c: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    b.replace(/\/$/, "") +
      "/api/companies/" +
      encodeURIComponent(c) +
      "/stock" +
      path,
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
export async function stockCards(
  b: string,
  c: string,
  signal?: AbortSignal,
): Promise<StockCard[]> {
  const v = await request(b, c, "/cards", undefined, signal);
  if (
    !Array.isArray(v) ||
    v.some(
      (x) =>
        typeof x.quantity !== "string" ||
        !x.item ||
        !Number.isSafeInteger(x.revision),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function stockMovements(
  b: string,
  c: string,
  q: Record<string, string>,
  signal?: AbortSignal,
): Promise<StockPage> {
  const v = await request(
    b,
    c,
    "/movements?" + new URLSearchParams(q),
    undefined,
    signal,
  );
  if (
    !Array.isArray(v?.items) ||
    !Number.isSafeInteger(v.total) ||
    (v.summary !== null &&
      (!v.summary ||
        ["opening", "received", "issued", "adjustments", "closing"].some(
          (k) =>
            typeof v.summary[k] !== "string" ||
            !/^-?\d+(\.\d+)?$/.test(v.summary[k]),
        )))
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function stockExecute(
  b: string,
  c: string,
  cmd: StockCommand,
): Promise<StockMovement> {
  const v = await request(b, c, "/commands", cmd);
  if (typeof v?.id !== "string" || typeof v.quantity !== "string")
    throw new SalesFailure(502, "invalid_response");
  return v;
}

export type StockReport = components["schemas"]["StockReport"];
export type StockCost = components["schemas"]["StockCost"];
export type StockCostCommand = components["schemas"]["StockCostCommand"];
export async function stockReport(
  b: string,
  c: string,
  q: Record<string, string>,
  signal?: AbortSignal,
): Promise<StockReport> {
  const v = await request(
    b,
    c,
    "/report?" + new URLSearchParams(q),
    undefined,
    signal,
  );
  if (
    !Array.isArray(v?.items) ||
    !Number.isSafeInteger(v.total) ||
    typeof v.known_value !== "string" ||
    v.items.some(
      (i: StockReport["items"][number]) =>
        typeof i.quantity !== "string" ||
        [
          i.value,
          i.issued_value,
          i.correction_stock_value,
          i.correction_issued_value,
        ].some(
          (v) =>
            v !== null && (typeof v !== "string" || !/^-?\d+(\.\d+)?$/.test(v)),
        ),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function stockCosts(
  b: string,
  c: string,
  id: string,
  signal?: AbortSignal,
): Promise<StockCost[]> {
  const v = await request(
    b,
    c,
    "/costs/" + encodeURIComponent(id),
    undefined,
    signal,
  );
  if (!Array.isArray(v)) throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function stockSetCost(
  b: string,
  c: string,
  cmd: StockCostCommand,
): Promise<StockCost> {
  const v = await request(b, c, "/costs", cmd);
  if (!Number.isSafeInteger(v?.revision))
    throw new SalesFailure(502, "invalid_response");
  return v;
}
