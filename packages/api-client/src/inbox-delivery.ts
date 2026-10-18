import type { components } from "./inbox-delivery.generated";
import { SalesFailure } from "./sales";
export type InboxOriginalSend = components["schemas"]["InboxSendOriginal"];
export type InboxOriginalDelivery = components["schemas"]["InboxDelivery"];
export type InboxOriginalHistory =
  components["schemas"]["InboxDeliveryHistory"];
function valid(v: unknown): v is InboxOriginalDelivery {
  if (!v || typeof v !== "object") return false;
  const x = v as Record<string, unknown>;
  return (
    [
      "id",
      "document_id",
      "recipient",
      "subject",
      "body",
      "filename",
      "sha256",
      "sender",
      "created_at",
    ].every((k) => typeof x[k] === "string") &&
    Number.isSafeInteger(x.revision) &&
    Number.isSafeInteger(x.size) &&
    ["queued", "sending", "sent", "failed", "uncertain"].includes(
      String(x.status),
    ) &&
    (x.error_code === null || typeof x.error_code === "string") &&
    (x.finished_at === null || typeof x.finished_at === "string")
  );
}
async function request(
  b: string,
  c: string,
  d: string,
  body?: InboxOriginalSend,
) {
  const r = await fetch(
    b.replace(/\/$/, "") +
      `/api/companies/${encodeURIComponent(c)}/inbox/${encodeURIComponent(d)}/deliveries`,
    {
      method: body ? "POST" : "GET",
      credentials: "include",
      signal: AbortSignal.timeout(30000),
      headers: body
        ? { "Content-Type": "application/json", "X-Faktura-Request": "1" }
        : undefined,
      body: body ? JSON.stringify(body) : undefined,
    },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  return r.json();
}
export async function inboxOriginalHistory(
  b: string,
  c: string,
  d: string,
): Promise<InboxOriginalHistory> {
  const v = await request(b, c, d);
  if (
    typeof v?.configured !== "boolean" ||
    !(v.sender === null || typeof v.sender === "string") ||
    !Array.isArray(v.items) ||
    v.items.length > 50 ||
    v.items.some((x: unknown) => !valid(x))
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function inboxOriginalSend(
  b: string,
  c: string,
  d: string,
  body: InboxOriginalSend,
): Promise<InboxOriginalDelivery> {
  const v = await request(b, c, d, body);
  if (!valid(v)) throw new SalesFailure(502, "invalid_response");
  return v;
}
