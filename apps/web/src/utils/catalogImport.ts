import type { ImportRecord, ImportTable } from "@open-faktura/api-client";
export const catalogImportFields = [
  "key",
  "name",
  "code",
  "barcode",
  "unit",
  "unit_price",
  "currency",
  "vat_percent",
] as const;
export type CatalogImportField = (typeof catalogImportFields)[number];
export type CatalogImportMapping = Record<CatalogImportField, number>;
export const requiredCatalogFields: CatalogImportField[] = [
  "key",
  "name",
  "unit_price",
  "currency",
  "vat_percent",
];
const aliases: Record<CatalogImportField, string[]> = {
  key: ["id", "source_id", "key"],
  name: ["name", "název", "nazev"],
  code: ["code", "kód", "kod"],
  barcode: ["barcode", "ean", "čárový kód"],
  unit: ["unit", "jednotka", "mj"],
  unit_price: ["unit_price", "cena", "price"],
  currency: ["currency", "měna", "mena"],
  vat_percent: ["vat_percent", "dph", "sazba dph"],
};
export function suggestCatalogMapping(headers: string[]): CatalogImportMapping {
  return Object.fromEntries(
    catalogImportFields.map((k) => [
      k,
      headers.findIndex((h) => aliases[k].includes(h.trim().toLowerCase())),
    ]),
  ) as CatalogImportMapping;
}
export function mapCatalogImport(
  table: ImportTable,
  mapping: CatalogImportMapping,
  priceMode: "net" | "gross",
  decimalSeparator: "." | ",",
): ImportRecord[] {
  return table.rows.map((row) => {
    const get = (k: CatalogImportField) => (row[mapping[k]] ?? "").trim();
    const decimal = (value: string) => {
      // No arithmetic, grouping removal, currency stripping or inferred zero.
      const pattern =
        decimalSeparator === "," ? /^\d+(,\d+)?$/ : /^\d+(\.\d+)?$/;
      return pattern.test(value) ? value.replace(",", ".") : "invalid";
    };
    return {
      kind: "catalog",
      key: get("key"),
      data: {
        name: get("name"),
        code: get("code"),
        barcode: get("barcode"),
        unit: get("unit"),
        unit_price: decimal(get("unit_price")),
        currency: get("currency"),
        vat_percent: decimal(get("vat_percent")),
        price_mode: priceMode,
        quantity: "1",
      },
    };
  });
}
