import type { components } from "./retail.generated";
import { SalesFailure } from "./sales";
export type RetailData = components["schemas"]["RetailData"];
export type RetailDocument = components["schemas"]["RetailDocument"];
export type RetailPreview = components["schemas"]["RetailPreview"];
export type RetailCommand = components["schemas"]["RetailCommand"];
export type RetailPage = components["schemas"]["RetailPage"];
const root = (b: string, c: string) =>
  `${b.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/retail`;
async function request(
  b: string,
  c: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(root(b, c) + path, {
    method: body === undefined ? "GET" : "POST",
    credentials: "include",
    headers:
      body === undefined
        ? undefined
        : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: signal ?? AbortSignal.timeout(30000),
  });
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  return r.json();
}
function preview(v: RetailPreview): RetailPreview {
  if (
    !v ||
    !v.totals ||
    typeof v.totals.payable !== "string" ||
    !Array.isArray(v.totals.lines) ||
    !Array.isArray(v.allocations)
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
function document(v: RetailDocument): RetailDocument {
  preview(v);
  if (
    typeof v.id !== "string" ||
    typeof v.number !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.cancelled !== "boolean" ||
    !v.data?.invoice ||
    !v.data.payments
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function retailList(
  b: string,
  c: string,
  q: string,
  cancelled: boolean,
  page: number,
  signal?: AbortSignal,
): Promise<RetailPage> {
  const v = await request(
    b,
    c,
    `?${new URLSearchParams({ query: q, cancelled: String(cancelled), page: String(page) })}`,
    undefined,
    signal,
  );
  if (!Array.isArray(v?.items) || !Number.isSafeInteger(v.total))
    throw new SalesFailure(502, "invalid_response");
  return { ...v, items: v.items.map(document) };
}
export async function retailGet(
  b: string,
  c: string,
  id: string,
  signal?: AbortSignal,
) {
  return document(
    await request(b, c, `/${encodeURIComponent(id)}`, undefined, signal),
  );
}
export async function retailPreview(
  b: string,
  c: string,
  data: RetailData,
  signal?: AbortSignal,
) {
  return preview(await request(b, c, "/preview", data, signal));
}
export async function retailExecute(b: string, c: string, data: RetailCommand) {
  return document(await request(b, c, "/commands", data));
}
export const retailPdfUrl = (b: string, c: string, id: string) =>
  root(b, c) + `/${encodeURIComponent(id)}/pdf`;
export type RetailPosting = components["schemas"]["Posting"];
export type RetailPostingInput = components["schemas"]["PostingInput"];
export type RetailPostingPreview = components["schemas"]["PostingPreview"];
function posting(v: RetailPosting): RetailPosting {
  if (
    !v ||
    typeof v.id !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    !Array.isArray(v.groups)
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function retailPostings(
  b: string,
  c: string,
  signal?: AbortSignal,
): Promise<RetailPosting[]> {
  const v = await request(b, c, "/postings", undefined, signal);
  if (!Array.isArray(v)) throw new SalesFailure(502, "invalid_response");
  return v.map(posting);
}
export async function retailPostingPreview(
  b: string,
  c: string,
  input: RetailPostingInput,
): Promise<RetailPostingPreview> {
  const v = await request(b, c, "/postings/preview", input);
  if (!Array.isArray(v?.groups) || typeof v.fingerprint !== "string")
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function retailPost(
  b: string,
  c: string,
  command: components["schemas"]["PostCommand"],
) {
  return posting(await request(b, c, "/postings", command));
}
export async function retailUnpost(
  b: string,
  c: string,
  command: components["schemas"]["UnpostCommand"],
) {
  return posting(await request(b, c, "/postings/reverse", command));
}
export type RetailAudit = components["schemas"]["RetailAudit"];
export async function retailHistory(
  b: string,
  c: string,
  id: string,
  signal?: AbortSignal,
): Promise<RetailAudit[]> {
  const v = await request(
    b,
    c,
    `/${encodeURIComponent(id)}/history`,
    undefined,
    signal,
  );
  if (
    !Array.isArray(v) ||
    v.some(
      (r) =>
        !Number.isSafeInteger(r.revision) ||
        typeof r.reason !== "string" ||
        typeof r.created_at !== "string",
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
