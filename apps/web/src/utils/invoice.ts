import {
  InvoiceCalculationFailure,
  type Company,
  type Contact,
  type DraftData,
  type PartySnapshot,
} from "@open-faktura/api-client";
import type { AppKey } from "../locales";
export const emptyParty = (): PartySnapshot => ({
  name: "",
  registration_number: "",
  vat_number: "",
  email: "",
  phone: "",
  address: { street: "", city: "", postal_code: "", country: "CZ" },
});
export const contactParty = (c: Contact): PartySnapshot => ({
  ...emptyParty(),
  name: c.data.name,
  registration_number: c.data.registration_number,
  vat_number: c.data.vat_number,
  email: c.data.email,
  phone: c.data.phone,
  address: { ...c.data.billing_address },
});
export function addDays(date: string, days: number) {
  const d = new Date(`${date}T12:00:00Z`);
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}
export function newLine(tax: DraftData["calculation"]["tax_mode"]) {
  return {
    id: crypto.randomUUID(),
    description: "",
    unit: "",
    quantity: "1",
    unit_price: "",
    discount_percent: "0",
    vat_percent: tax === "vat_payer" ? ("21" as const) : ("0" as const),
  };
}
export function newDraft(company: Company, language: "cs" | "en"): DraftData {
  const s = company.settings;
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: s.timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(new Date());
  const part = (k: string) => parts.find((p) => p.type === k)!.value;
  const today = `${part("year")}-${part("month")}-${part("day")}`;
  return {
    contact_id: null,
    supplier: {
      name: s.name,
      registration_number: s.registration_number,
      vat_number: s.vat_number,
      email: s.email,
      phone: s.phone,
      address: {
        street: s.street,
        city: s.city,
        postal_code: s.postal_code,
        country: s.country,
      },
    },
    customer: null,
    payment: {
      method: s.payment_method,
      bank_account: "",
      iban: s.iban,
      variable_symbol: "",
    },
    calculation: {
      calculation_version: "of-czk-v1",
      currency: "CZK",
      tax_mode: s.tax_mode,
      price_mode: "net",
      discount_percent: "0",
      rounding: "none",
      lines: [newLine(s.tax_mode)],
    },
    issued_on: today,
    due_on: addDays(today, s.maturity_days),
    tax_on: s.tax_mode === "vat_payer" ? today : null,
    language,
    order_number: "",
    note: "",
  };
}
export function invoiceError(error: unknown): AppKey {
  if (error instanceof InvoiceCalculationFailure) {
    if (error.fields.includes("advance_tax_exchange_rate"))
      return "settlementFxTaxUnsupported";
    if (error.status === 409) return "draftConflict";
    if (error.status === 404) return "draftNotFound";
    if (error.status === 403) return "companyForbidden";
    if (error.status === 422) return "draftInvalid";
  }
  return "requestFailed";
}
export function moneyText(value: string, language: string, currency = "CZK") {
  const [whole, fraction = "00"] = value.split(".");
  return `${whole === "-0" ? "−" : ""}${BigInt(whole).toLocaleString(language === "cs" ? "cs-CZ" : "en-GB")}${language === "cs" ? "," : "."}${fraction.padEnd(2, "0")} ${currency === "CZK" && language === "cs" ? "Kč" : currency}`;
}
