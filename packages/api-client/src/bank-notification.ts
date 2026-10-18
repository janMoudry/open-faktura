import type { components } from "./bank-notification.generated";
import { SalesFailure } from "./sales";
export type BankNotificationSettings =
  components["schemas"]["BankNotificationSettings"];
export type BankNotificationDiagnostic =
  components["schemas"]["BankNotificationDiagnostic"];
async function request(
  b: string,
  c: string,
  account: string,
  path: string,
  body?: unknown,
) {
  const response = await fetch(
    b.replace(/\/$/, "") +
      `/api/companies/${encodeURIComponent(c)}/bank/accounts/${encodeURIComponent(account)}/notification-${path}`,
    {
      method: body === undefined ? "GET" : "POST",
      credentials: "include",
      signal: AbortSignal.timeout(30000),
      headers:
        body === undefined
          ? undefined
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
    },
  );
  if (!response.ok) {
    const value = await response.json().catch(() => null);
    throw new SalesFailure(
      response.status,
      value?.code ?? "unavailable",
      value?.fields ?? [],
    );
  }
  return response.json();
}
export async function bankNotificationSettings(
  b: string,
  c: string,
  account: string,
): Promise<BankNotificationSettings> {
  const v = await request(b, c, account, "settings");
  if (
    !Number.isSafeInteger(v?.revision) ||
    typeof v.enabled !== "boolean" ||
    typeof v.format !== "string"
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function bankNotificationConfigure(
  b: string,
  c: string,
  v: BankNotificationSettings,
): Promise<BankNotificationSettings> {
  return request(b, c, v.account_id, "settings", v);
}
export async function bankNotificationHistory(
  b: string,
  c: string,
  account: string,
): Promise<BankNotificationDiagnostic[]> {
  const v = await request(b, c, account, "history");
  if (!Array.isArray(v)) throw new SalesFailure(502, "invalid_response");
  return v;
}
