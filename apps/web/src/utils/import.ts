import type { ImportRecord, ImportTable } from "@open-faktura/api-client";
import { emptyContact } from "./contact";
export const importFields = [
  "key",
  "name",
  "registration_number",
  "vat_number",
  "street",
  "city",
  "postal_code",
  "country",
  "email",
  "phone",
  "bank_account",
  "iban",
  "note",
] as const;
export type ImportField = (typeof importFields)[number];
const aliases: Record<ImportField, string[]> = {
  key: ["id", "source_id", "id kontaktu"],
  name: ["název", "firma", "jméno", "název firmy", "name"],
  registration_number: ["ič", "ičo", "ico", "registration_number"],
  vat_number: ["dič", "dic", "vat_number"],
  street: ["ulice", "adresa", "street"],
  city: ["město", "city"],
  postal_code: ["psč", "psc", "postal_code"],
  country: ["země", "stát", "country"],
  email: ["e-mail", "email"],
  phone: ["telefon", "phone"],
  bank_account: ["bankovní účet", "bank_account"],
  iban: ["iban"],
  note: ["poznámka", "note"],
};
export function suggestImportMapping(headers: string[]) {
  return Object.fromEntries(
    importFields.map((k) => [
      k,
      headers.findIndex((h) => aliases[k].includes(h.trim().toLowerCase())),
    ]),
  ) as Record<ImportField, number>;
}
export function mapImportContacts(
  table: ImportTable,
  mapping: Record<ImportField, number>,
): ImportRecord[] {
  return table.rows.map((row) => {
    const get = (k: ImportField) => row[mapping[k]] ?? "";
    const data = emptyContact();
    for (const k of [
      "name",
      "registration_number",
      "vat_number",
      "email",
      "phone",
      "bank_account",
      "iban",
      "note",
    ] as const)
      data[k] = get(k);
    data.billing_address = {
      street: get("street"),
      city: get("city"),
      postal_code: get("postal_code"),
      country: get("country"),
    };
    return { kind: "contact", key: get("key"), data };
  });
}
export function saveImportReport(value: unknown, id: string) {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(value, null, 2)], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = `import-${id}.json`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
