import { InvoiceCalculationFailure } from "./invoice";
export async function invoiceIsdoc(
  base: string,
  company: string,
  id: string,
): Promise<Blob> {
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/invoices/${encodeURIComponent(id)}/isdoc`,
    { credentials: "include", signal: AbortSignal.timeout(15000) },
  );
  if (!response.ok) {
    const value = await response.json().catch(() => null);
    throw new InvoiceCalculationFailure(
      response.status,
      value?.code ?? "unavailable",
    );
  }
  return response.blob();
}
