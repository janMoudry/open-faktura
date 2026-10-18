import type { components } from "./inbox-mailbox.generated";
import { SalesFailure } from "./sales";
export type MailboxSettings = components["schemas"]["MailboxSettings"];
export type MailboxRun = components["schemas"]["MailboxRun"];
async function request(b: string, c: string, path: string, body?: unknown) {
  const r = await fetch(
    b.replace(/\/$/, "") +
      `/api/companies/${encodeURIComponent(c)}/inbox/mailbox` +
      path,
    {
      method: body === undefined ? "GET" : "POST",
      credentials: "include",
      signal: AbortSignal.timeout(180000),
      headers:
        body === undefined
          ? undefined
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
    },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  return r.json();
}
export async function inboxMailboxSettings(
  b: string,
  c: string,
): Promise<MailboxSettings> {
  const v = await request(b, c, "");
  if (
    !Number.isSafeInteger(v?.revision) ||
    typeof v.enabled !== "boolean" ||
    typeof v.automatic !== "boolean" ||
    !Number.isSafeInteger(v.cursor) ||
    !(v.provider === null || typeof v.provider === "string")
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function inboxMailboxConfigure(
  b: string,
  c: string,
  v: components["schemas"]["MailboxConfigure"],
): Promise<MailboxSettings> {
  return request(b, c, "", v);
}
export async function inboxMailboxSync(
  b: string,
  c: string,
  request_id: string,
): Promise<MailboxRun> {
  return request(b, c, "/sync", { request_id });
}
export async function inboxMailboxHistory(
  b: string,
  c: string,
): Promise<MailboxRun[]> {
  const v = await request(b, c, "/history");
  if (!Array.isArray(v)) throw new SalesFailure(502, "invalid_response");
  return v;
}
