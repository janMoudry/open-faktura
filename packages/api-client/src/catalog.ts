import type { components } from "./catalog.generated";
export type Catalog = components["schemas"]["Catalog"];
export type CatalogData = components["schemas"]["CatalogData"];
export type CatalogPage = components["schemas"]["CatalogPage"];
export class CatalogFailure extends Error {
  constructor(
    public status: number,
    public code: string,
    public fields: string[] = [],
  ) {
    super(code);
  }
}
function parseCatalog(value: unknown): Catalog {
  if (!value || typeof value !== "object")
    throw new CatalogFailure(502, "invalid_response");
  const c = value as Catalog;
  const d = c.data;
  if (
    typeof c.id !== "string" ||
    typeof c.company_id !== "string" ||
    !Number.isSafeInteger(c.revision) ||
    typeof c.archived !== "boolean" ||
    !d ||
    !["name", "code", "barcode", "unit", "quantity", "unit_price"].every(
      (key) => typeof (d as Record<string, unknown>)[key] === "string",
    ) ||
    d.currency !== "CZK" ||
    !["net", "gross"].includes(d.price_mode) ||
    !["0", "12", "21"].includes(d.vat_percent)
  )
    throw new CatalogFailure(502, "invalid_response");
  return c;
}
async function request(
  base: string,
  company: string,
  path: string,
  method = "GET",
  body?: unknown,
  signal?: AbortSignal,
) {
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/catalog${path}`,
    {
      method,
      credentials: "include",
      signal: signal ?? AbortSignal.timeout(15000),
      headers:
        body === undefined
          ? {}
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
    },
  );
  const value = await response.json().catch(() => null);
  if (!response.ok)
    throw new CatalogFailure(
      response.status,
      value?.code ?? "unavailable",
      Array.isArray(value?.fields) ? value.fields : [],
    );
  return value;
}
export async function listCatalogs(
  base: string,
  company: string,
  search: {
    query: string;
    archived: boolean;
    page: number;
    page_size?: number;
  },
  signal?: AbortSignal,
): Promise<CatalogPage> {
  const params = new URLSearchParams({
    query: search.query,
    archived: String(search.archived),
    page: String(search.page),
    page_size: String(search.page_size ?? 25),
  });
  const value = await request(
    base,
    company,
    `?${params}`,
    "GET",
    undefined,
    signal,
  );
  if (
    !value ||
    !Array.isArray(value.items) ||
    ![value.total, value.page, value.page_size].every(Number.isSafeInteger)
  )
    throw new CatalogFailure(502, "invalid_response");
  return { ...value, items: value.items.map(parseCatalog) };
}
export async function getCatalog(
  base: string,
  company: string,
  id: string,
  signal?: AbortSignal,
) {
  return parseCatalog(
    await request(
      base,
      company,
      `/${encodeURIComponent(id)}`,
      "GET",
      undefined,
      signal,
    ),
  );
}
export async function createCatalog(
  base: string,
  company: string,
  body: components["schemas"]["CreateCatalog"],
  signal?: AbortSignal,
) {
  return parseCatalog(await request(base, company, "", "POST", body, signal));
}
export async function updateCatalog(
  base: string,
  company: string,
  id: string,
  body: components["schemas"]["UpdateCatalog"],
  signal?: AbortSignal,
) {
  return parseCatalog(
    await request(
      base,
      company,
      `/${encodeURIComponent(id)}`,
      "PUT",
      body,
      signal,
    ),
  );
}
export async function catalogLine(
  base: string,
  company: string,
  id: string,
  options: { price_mode: "net" | "gross"; tax_mode: "non_payer" | "vat_payer" },
  signal?: AbortSignal,
): Promise<components["schemas"]["InvoiceLine"]> {
  const value = await request(
    base,
    company,
    `/${encodeURIComponent(id)}/line?${new URLSearchParams(options)}`,
    "GET",
    undefined,
    signal,
  );
  if (
    !value ||
    ![
      "id",
      "description",
      "unit",
      "quantity",
      "unit_price",
      "discount_percent",
      "vat_percent",
    ].every((key) => typeof value[key] === "string")
  )
    throw new CatalogFailure(502, "invalid_response");
  return value;
}
