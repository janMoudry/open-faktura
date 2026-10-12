import { ContactFailure, type ContactData } from "@open-faktura/api-client";
import type { AppKey } from "../locales";
export const emptyContact = (): ContactData => ({
  name: "",
  registration_number: "",
  vat_number: "",
  billing_address: { street: "", city: "", postal_code: "", country: "CZ" },
  delivery_addresses: [],
  default_delivery_id: null,
  email: "",
  phone: "",
  persons: [],
  recipients: [],
  maturity_days: null,
  payment_method: "",
  bank_account: "",
  iban: "",
  note: "",
});
export function contactError(error: unknown): AppKey {
  if (error instanceof ContactFailure) {
    if (error.status === 409) return "contactConflict";
    if (error.status === 404) return "contactNotFound";
    if (error.status === 403) return "companyForbidden";
    if (error.status === 422) return "contactInvalid";
  }
  return "requestFailed";
}
