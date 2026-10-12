import type { components } from "./invoice-delivery.generated";
import { SalesFailure } from "./sales";
export type SendInvoice = components["schemas"]["SendInvoice"];
export type InvoiceDeliveryHistory = components["schemas"]["DeliveryHistory"];
async function response(
  base: string,
  c: string,
  id: string,
  suffix: string,
  method = "GET",
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/invoices/${encodeURIComponent(id)}${suffix}`,
    {
      method,
      credentials: "include",
      signal: signal ?? AbortSignal.timeout(30000),
      headers:
        method === "GET"
          ? undefined
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
    },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  return r;
}
export async function invoiceHistory(
  base: string,
  c: string,
  id: string,
  page = 1,
  signal?: AbortSignal,
): Promise<InvoiceDeliveryHistory> {
  const v = await (
    await response(
      base,
      c,
      id,
      `/deliveries?page=${page}`,
      "GET",
      undefined,
      signal,
    )
  ).json();
  if (
    !v ||
    !Array.isArray(v.items) ||
    typeof v.configured !== "boolean" ||
    !Number.isSafeInteger(v.total)
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function sendInvoice(
  base: string,
  c: string,
  id: string,
  cmd: SendInvoice,
): Promise<components["schemas"]["Delivery"]> {
  const v = await (
    await response(base, c, id, "/deliveries", "POST", cmd)
  ).json();
  if (!v || typeof v.id !== "string" || typeof v.status !== "string")
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function invoiceDeliveryPdf(
  base: string,
  c: string,
  id: string,
  delivery?: string,
): Promise<Blob> {
  return (
    await response(
      base,
      c,
      id,
      delivery ? `/deliveries/${encodeURIComponent(delivery)}/pdf` : "/pdf",
    )
  ).blob();
}
