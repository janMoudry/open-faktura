import type { components } from "./taxpayer-filing-profile.generated";
import { SalesFailure } from "./sales";
export { epoOffices } from "./epo-offices";
export type TaxpayerProfile = components["schemas"]["Profile"];
export type TaxpayerProfileCommand = components["schemas"]["Command"];
export type TaxpayerProfileData = components["schemas"]["Data"];
export async function taxpayerFilingProfile(
  base: string,
  company: string,
  input?: TaxpayerProfileCommand,
): Promise<TaxpayerProfile> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/reports/tax/filing-profile`,
    {
      credentials: "include",
      signal: AbortSignal.timeout(30000),
      ...(input
        ? {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "X-Faktura-Request": "1",
            },
            body: JSON.stringify(input),
          }
        : {}),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "unavailable");
  const v: unknown = await r.json();
  if (
    !v ||
    typeof v !== "object" ||
    !("revision" in v) ||
    !Number.isSafeInteger(v.revision) ||
    !("data" in v)
  )
    throw new Error("Invalid taxpayer profile");
  const d = v.data;
  if (
    d !== null &&
    (!d ||
      typeof d !== "object" ||
      ![
        "tax_office",
        "taxpayer_number",
        "taxpayer_kind",
        "vat_filer_kind",
      ].every(
        (k) => k in d && typeof (d as Record<string, unknown>)[k] === "string",
      ) ||
      !("tax_office_workplace" in d) ||
      (d.tax_office_workplace !== null &&
        typeof d.tax_office_workplace !== "string"))
  )
    throw new Error("Invalid taxpayer profile");
  return v as TaxpayerProfile;
}
