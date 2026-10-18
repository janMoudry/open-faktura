import type { components } from "./communication.generated";
import { SalesFailure } from "./sales";
export type CommunicationSettings = components["schemas"]["Settings"];
export type CommunicationPreview = components["schemas"]["Preview"];
export type CommunicationPurpose = CommunicationPreview["purpose"];
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
      method: body === undefined ? "GET" : "PUT",
      credentials: "include",
      signal: signal ?? AbortSignal.timeout(30000),
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
function settings(v: CommunicationSettings) {
  if (
    !v ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.reminders !== "boolean" ||
    typeof v.thanks !== "boolean" ||
    typeof v.minimum_total !== "string"
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function getCommunicationSettings(
  b: string,
  c: string,
  signal?: AbortSignal,
): Promise<CommunicationSettings> {
  return settings(await request(b, c, "/communication", undefined, signal));
}
export async function saveCommunicationSettings(
  b: string,
  c: string,
  v: CommunicationSettings,
): Promise<CommunicationSettings> {
  return settings(await request(b, c, "/communication", v));
}
export async function previewCommunication(
  b: string,
  c: string,
  id: string,
  purpose: CommunicationPurpose,
  signal?: AbortSignal,
): Promise<CommunicationPreview> {
  const v = await request(
    b,
    c,
    `/invoices/${encodeURIComponent(id)}/communication/${purpose}`,
    undefined,
    signal,
  );
  if (
    !v ||
    typeof v.eligible !== "boolean" ||
    typeof v.paused !== "boolean" ||
    typeof v.subject !== "string" ||
    typeof v.body !== "string" ||
    !Number.isSafeInteger(v.settlement_revision)
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function pauseCommunication(
  b: string,
  c: string,
  id: string,
  paused: boolean,
): Promise<{ paused: boolean }> {
  return request(b, c, `/invoices/${encodeURIComponent(id)}/communication`, {
    paused,
  });
}
