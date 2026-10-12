import { CatalogFailure, type CatalogData } from "@open-faktura/api-client";
import type { AppKey } from "../locales";
export const emptyCatalog = (): CatalogData => ({
  name: "",
  code: "",
  barcode: "",
  unit: "ks",
  quantity: "1",
  unit_price: "0",
  currency: "CZK",
  price_mode: "net",
  vat_percent: "0",
});
export function catalogError(error: unknown): AppKey {
  if (error instanceof CatalogFailure) {
    if (error.status === 409) return "catalogConflict";
    if (error.status === 404) return "catalogNotFound";
    if (error.status === 403) return "companyForbidden";
    if (error.status === 422) return "catalogInvalid";
  }
  return "requestFailed";
}
