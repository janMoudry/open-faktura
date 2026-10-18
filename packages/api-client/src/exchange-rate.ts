import type { components } from "./exchange-rate.generated";
import { InvoiceCalculationFailure, isDocumentCurrency } from "./invoice";
export type RateProposal = components["schemas"]["Rate"];
export async function lookupExchangeRate(
  base: string,
  company: string,
  currency: string,
  date: string,
  signal?: AbortSignal,
): Promise<RateProposal> {
  const params = new URLSearchParams({ currency, date });
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/exchange-rates?${params}`,
    { credentials: "include", signal: signal ?? AbortSignal.timeout(15000) },
  );
  const value = await response.json().catch(() => null);
  if (!response.ok)
    throw new InvoiceCalculationFailure(
      response.status,
      value?.code ?? "unavailable",
    );
  if (
    !value ||
    !isDocumentCurrency(value.currency) ||
    value.currency !== currency ||
    value.requested_on !== date ||
    !/^\d{4}-\d{2}-\d{2}$/.test(value.published_on) ||
    value.source !== "CNB" ||
    typeof value.exchange_rate?.rate !== "string" ||
    !/^\d+(\.\d{1,6})?$/.test(value.exchange_rate.rate) ||
    typeof value.exchange_rate?.units !== "string" ||
    !/^\d+$/.test(value.exchange_rate.units)
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return value;
}
