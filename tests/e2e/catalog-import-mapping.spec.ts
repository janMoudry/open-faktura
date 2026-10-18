import { test, expect } from "@playwright/test";
import {
  mapCatalogImport,
  suggestCatalogMapping,
} from "../../apps/web/src/utils/catalogImport";
test("catalog mapping preserves identifiers and exact decimals; ambiguous prices stay invalid", () => {
  const headers = [
    "id",
    "name",
    "code",
    "barcode",
    "unit",
    "unit_price",
    "currency",
    "vat_percent",
  ];
  const mapping = suggestCatalogMapping(headers);
  const table = {
    headers,
    rows: [["0001", "Service", "0002", "0003", "h", "1250,5000", "CZK", "21"]],
  };
  const result = mapCatalogImport(table, mapping, "gross", ",");
  expect(result).toEqual([
    {
      kind: "catalog",
      key: "0001",
      data: {
        name: "Service",
        code: "0002",
        barcode: "0003",
        unit: "h",
        unit_price: "1250.5000",
        currency: "CZK",
        vat_percent: "21",
        quantity: "1",
        price_mode: "gross",
      },
    },
  ]);
  for (const price of [
    "",
    "1,234.56",
    "1 234,56",
    "12 Kč",
    "NaN",
    "=1+1",
    "1e3",
    "-1",
  ]) {
    const rows = [["0001", "Service", "", "", "h", price, "CZK", "21"]];
    const [record] = mapCatalogImport({ headers, rows }, mapping, "net", ",");
    expect(record.kind).toBe("catalog");
    if (record.kind === "catalog")
      expect(record.data.unit_price).toBe("invalid");
  }
  const [wrongSeparator] = mapCatalogImport(table, mapping, "net", ".");
  if (wrongSeparator.kind === "catalog")
    expect(wrongSeparator.data.unit_price).toBe("invalid");
});
