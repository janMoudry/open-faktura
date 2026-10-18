import type { components } from "./inbox-notification.generated";
import { SalesFailure } from "./sales";
export type NotificationFeed = components["schemas"]["NotificationFeed"];
export type NotificationConfigure =
  components["schemas"]["NotificationConfigure"];
async function request(
  b: string,
  c: string,
  path: string,
  body?: unknown,
): Promise<NotificationFeed> {
  const r = await fetch(
    b.replace(/\/$/, "") +
      `/api/companies/${encodeURIComponent(c)}/inbox/notifications` +
      path,
    {
      method: body === undefined ? "GET" : "POST",
      credentials: "include",
      signal: AbortSignal.timeout(15000),
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
  const v = await r.json();
  if (
    !v ||
    !Number.isSafeInteger(v.unread) ||
    v.unread < 0 ||
    typeof v.through !== "string" ||
    !/^\d+$/.test(v.through) ||
    !Number.isSafeInteger(v.settings?.revision) ||
    typeof v.settings.new_documents !== "boolean" ||
    typeof v.settings.errors !== "boolean" ||
    !Array.isArray(v.events) ||
    v.events.length > 50 ||
    v.events.some((e: unknown) => {
      if (!e || typeof e !== "object") return true;
      const x = e as Record<string, unknown>;
      return (
        typeof x.id !== "string" ||
        !/^\d+$/.test(x.id) ||
        !["document", "error"].includes(String(x.category)) ||
        typeof x.unread !== "boolean" ||
        typeof x.created_at !== "string" ||
        Number.isNaN(Date.parse(x.created_at)) ||
        !(x.document_id === null || typeof x.document_id === "string") ||
        !(x.filename === null || typeof x.filename === "string") ||
        ![
          null,
          "mailbox_failed",
          "message_rejected",
          "gateway_failed",
        ].includes(x.code as null | string)
      );
    })
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export const inboxNotificationFeed = (b: string, c: string) =>
  request(b, c, "");
export const inboxNotificationConfigure = (
  b: string,
  c: string,
  v: NotificationConfigure,
) => request(b, c, "/settings", v);
export const inboxNotificationRead = (b: string, c: string, through: string) =>
  request(b, c, "/read", { through });
