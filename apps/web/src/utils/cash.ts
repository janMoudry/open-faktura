import { SalesFailure } from "@open-faktura/api-client";
import type { AppKey } from "../locales";
export function cashError(e: unknown): AppKey {
  if (e instanceof SalesFailure) {
    const taxErrors = {
      cash_tax_payer: "cashTaxPayer",
      cash_tax_date: "cashTaxInvalidDate",
      cash_tax_amount: "cashTaxInvalidAmount",
      cash_tax_description: "cashTaxDescription",
      cash_tax_match: "cashTaxNoMatch",
    } as const;
    for (const [key, label] of Object.entries(taxErrors))
      if (e.fields.includes(key)) return label;

    if (e.fields.includes("cash_default_archived"))
      return "cashDefaultArchived";
    if (e.fields.includes("cash_automatic_source"))
      return "cashAutomaticSource";
    if (e.fields.includes("cash_automatic_date")) return "cashAutomaticDate";
    if (e.status === 409) return "cashConflict";
    if (e.status === 422) return "cashInvalid";
  }
  return "cashFailed";
}
