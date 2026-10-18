import type { components } from "./vies.generated";
import { InvoiceCalculationFailure } from "./invoice";
export type ViesObservation = components["schemas"]["Observation"];
export const EU_VAT_PREFIXES = [
  "AT",
  "BE",
  "BG",
  "HR",
  "CY",
  "CZ",
  "DK",
  "EE",
  "FI",
  "FR",
  "DE",
  "EL",
  "HU",
  "IE",
  "IT",
  "LV",
  "LT",
  "LU",
  "MT",
  "NL",
  "PL",
  "PT",
  "RO",
  "SK",
  "SI",
  "ES",
  "SE",
  "XI",
];
export async function lookupVies(
  base: string,
  company: string,
  vatNumber: string,
): Promise<ViesObservation> {
  const params = new URLSearchParams({ vat_number: vatNumber });
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/vies?${params}`,
    { credentials: "include", signal: AbortSignal.timeout(15000) },
  );
  const v = await r.json().catch(() => null);
  if (!r.ok)
    throw new InvoiceCalculationFailure(r.status, v?.code ?? "unavailable");
  if (
    !v ||
    v.source !== "EU_VIES" ||
    typeof v.valid !== "boolean" ||
    typeof v.name !== "string" ||
    typeof v.address !== "string" ||
    typeof v.requested_on !== "string"
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return v;
}
