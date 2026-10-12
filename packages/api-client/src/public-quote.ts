import type { components } from "./public-quote.generated";
import { SalesFailure } from "./sales";
export type PublicQuote = components["schemas"]["PublicQuote"];
async function request(
  base: string,
  action: string,
  body: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/public/quotes/${action}`,
    {
      method: "POST",
      credentials: "omit",
      referrerPolicy: "no-referrer",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      signal: signal ?? AbortSignal.timeout(30000),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "public_quote_unavailable");
  return r;
}
export async function getPublicQuote(
  base: string,
  token: string,
  signal?: AbortSignal,
): Promise<PublicQuote> {
  const r = await request(base, "resolve", { token }, signal);
  const v = await r.json();
  if (
    !v ||
    typeof v.number !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    !v.invoice?.supplier ||
    !v.invoice?.customer ||
    !v.totals?.payable
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function publicQuotePdf(
  base: string,
  token: string,
): Promise<Blob> {
  return (await request(base, "pdf", { token })).blob();
}
export async function recordQuoteView(
  base: string,
  token: string,
  visit_id: string,
  revision: number,
  signal?: AbortSignal,
): Promise<void> {
  await request(base, "view", { token, visit_id, revision }, signal);
}
