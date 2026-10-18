import type { components } from "./invoice.generated";
export const documentCurrencies = [
  "CZK",
  "EUR",
  "USD",
  "GBP",
  "CHF",
  "PLN",
  "HUF",
  "JPY",
  "SEK",
  "NOK",
  "DKK",
  "CAD",
  "AUD",
] as const;
export const isDocumentCurrency = (value: unknown): boolean =>
  typeof value === "string" &&
  (documentCurrencies as readonly string[]).includes(value);
export type InvoiceCalculation = components["schemas"]["InvoiceCalculation"];
export type InvoiceTotals = components["schemas"]["InvoiceTotals"];
export class InvoiceCalculationFailure extends Error {
  constructor(
    public status: number,
    public code: string,
    public fields: string[] = [],
  ) {
    super(code);
  }
}
export async function calculateInvoice(
  base: string,
  company: string,
  input: InvoiceCalculation,
  signal?: AbortSignal,
): Promise<InvoiceTotals> {
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/invoices/calculate`,
    {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: JSON.stringify(input),
      signal: signal ?? AbortSignal.timeout(15000),
    },
  );
  const value = await response.json().catch(() => null);
  if (!response.ok)
    throw new InvoiceCalculationFailure(
      response.status,
      value?.code ?? "unavailable",
      Array.isArray(value?.fields) ? value.fields : [],
    );
  return parseInvoiceTotals(value);
}
export function parseInvoiceTotals(value: any): InvoiceTotals {
  const money = (v: unknown) =>
    typeof v === "string" && /^-?\d+\.\d{2}$/.test(v);
  const amounts = (v: any) => v && [v.net, v.vat, v.gross].every(money);
  if (
    !value ||
    !["of-czk-v1", "import-v1"].includes(value.calculation_version) ||
    !isDocumentCurrency(value.currency) ||
    !amounts(value.amounts) ||
    !money(value.rounding) ||
    !money(value.payable) ||
    !Array.isArray(value.lines) ||
    !value.lines.every(
      (l: any) => typeof l?.id === "string" && amounts(l.amounts),
    ) ||
    !Array.isArray(value.tax_summary) ||
    !value.tax_summary.every(
      (s: any) =>
        ["non_payer", "vat_payer"].includes(s?.tax_mode) &&
        ["0", "12", "21"].includes(s.vat_percent) &&
        amounts(s.amounts),
    )
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return value;
}
