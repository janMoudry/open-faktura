import type { components } from "./inbox.generated";
import { SalesFailure } from "./sales";
export type InboxDocument = components["schemas"]["InboxDocument"];
export type InboxPage = components["schemas"]["InboxPage"];
export type InboxCommand = components["schemas"]["InboxCommand"];
const root = (b: string, c: string) =>
  b.replace(/\/$/, "") + "/api/companies/" + encodeURIComponent(c) + "/inbox";
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
function document(v: InboxDocument) {
  if (
    !v ||
    typeof v.id !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.filename !== "string" ||
    typeof v.archived !== "boolean"
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function inboxList(
  b: string,
  c: string,
  q: string,
  archived: boolean,
  page: number,
  signal?: AbortSignal,
): Promise<InboxPage> {
  const v = await request(
    b,
    c,
    "?" +
      new URLSearchParams({
        query: q,
        archived: String(archived),
        page: String(page),
      }),
    undefined,
    signal,
  );
  if (!Array.isArray(v?.items) || !Number.isSafeInteger(v.total))
    throw new SalesFailure(502, "invalid_response");
  return { ...v, items: v.items.map(document) };
}
export async function inboxUpload(
  b: string,
  c: string,
  input: components["schemas"]["InboxUpload"],
) {
  return document(await request(b, c, "", input));
}
export async function inboxExecute(b: string, c: string, input: InboxCommand) {
  return document(await request(b, c, "/commands", input));
}
export const inboxFileUrl = (b: string, c: string, id: string) =>
  root(b, c) + "/" + encodeURIComponent(id) + "/file";

export async function inboxExtract(
  b: string,
  c: string,
  id: string,
): Promise<components["schemas"]["ReceivedData"]> {
  const v = await request(b, c, `/${encodeURIComponent(id)}/extract`);
  if (!v?.supplier || !Array.isArray(v.lines) || v.currency !== "CZK")
    throw new SalesFailure(502, "invalid_response");
  return v;
}

export type InboxAudit = components["schemas"]["InboxAudit"];
export async function inboxHistory(
  b: string,
  c: string,
  id: string,
): Promise<InboxAudit[]> {
  const v = await request(b, c, `/${encodeURIComponent(id)}/history`);
  if (!Array.isArray(v)) throw new SalesFailure(502, "invalid_response");
  return v;
}
