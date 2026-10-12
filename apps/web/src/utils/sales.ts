import { SalesFailure } from "@open-faktura/api-client";
import type { AppKey } from "../locales";
export const salesStates: Record<string, AppKey> = {
  created: "salesCreated",
  sent: "salesSent",
  ordered: "salesOrdered",
  invoiced: "salesInvoiced",
  rejected: "salesRejected",
};
export function salesError(e: unknown): AppKey {
  if (e instanceof SalesFailure) {
    if (e.status === 409) return "salesConflict";
    if (e.status === 404) return "salesNotFound";
    if (e.status === 403) return "companyForbidden";
    if (e.status === 422) return "salesInvalid";
  }
  return "requestFailed";
}
