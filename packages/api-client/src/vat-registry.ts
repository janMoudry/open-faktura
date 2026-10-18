import type { components } from "./vat-registry.generated";
import { InvoiceCalculationFailure } from "./invoice";
export type VatObservation = components["schemas"]["Observation"];
export async function lookupVatRegistry(
  base: string,
  company: string,
  vatNumber: string,
): Promise<VatObservation> {
  const params = new URLSearchParams({ vat_number: vatNumber });
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/vat-registry?${params}`,
    { credentials: "include", signal: AbortSignal.timeout(15000) },
  );
  const value = await response.json().catch(() => null);
  if (!response.ok)
    throw new InvoiceCalculationFailure(
      response.status,
      value?.code ?? "unavailable",
    );
  if (
    !value ||
    value.source !== "CZ_RWS" ||
    !Array.isArray(value.accounts) ||
    ![
      "vat_payer",
      "vat_group",
      "identified_person",
      "unreliable_person",
      "not_found",
    ].includes(value.subject_type) ||
    !["reliable", "unreliable", "not_found"].includes(value.reliability)
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return value;
}
