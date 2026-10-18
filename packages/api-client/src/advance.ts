import type { components } from "./advance.generated";
import { SalesFailure } from "./sales";
export type AdvanceReceipt = components["schemas"]["Receipt"];
export type AdvancePayment = components["schemas"]["Register"];
async function request(
  base: string,
  c: string,
  d: string,
  path: string,
  body?: unknown,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/advances/${encodeURIComponent(d)}${path}`,
    {
      credentials: "include",
      method: body === undefined ? "GET" : "POST",
      headers:
        body === undefined
          ? undefined
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(30000),
    },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  return r;
}
export async function advancePdf(b: string, c: string, d: string) {
  return (await request(b, c, d, "/pdf")).blob();
}
export async function advancePayments(
  b: string,
  c: string,
  d: string,
): Promise<AdvanceReceipt[]> {
  return (await request(b, c, d, "/payments")).json();
}
export async function payAdvance(
  b: string,
  c: string,
  d: string,
  cmd: AdvancePayment,
) {
  await request(b, c, d, "/payments", cmd);
}
export async function reverseAdvance(
  b: string,
  c: string,
  d: string,
  p: string,
  cmd: components["schemas"]["Reverse"],
) {
  await request(b, c, d, `/payments/${encodeURIComponent(p)}/reverse`, cmd);
}
