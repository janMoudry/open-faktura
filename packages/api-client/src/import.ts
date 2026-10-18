import type { components } from "./import.generated";
import { InvoiceCalculationFailure } from "./invoice";
export type ImportBatch = components["schemas"]["ImportBatch"];
export type ImportRecord = components["schemas"]["ImportRecord"];
export type ImportPrepare = components["schemas"]["PrepareImport"];
export type ImportTable = components["schemas"]["ImportTable"];
function parse(v: any): ImportBatch {
  if (
    !v ||
    ![v.id, v.source, v.agenda].every((x) => typeof x === "string") ||
    !["preview", "running", "done"].includes(v.status) ||
    !Array.isArray(v.limitations) ||
    !v.limitations.every((x: unknown) => typeof x === "string") ||
    !Array.isArray(v.rows) ||
    !Array.isArray(v.totals) ||
    !v.rows.every(
      (r: any) =>
        Number.isSafeInteger(r.index) &&
        ["contact", "invoice", "catalog"].includes(r.kind) &&
        [r.key, r.label].every((x) => typeof x === "string") &&
        [
          "ready",
          "invalid",
          "conflict",
          "duplicate",
          "imported",
          "skipped",
        ].includes(r.status) &&
        Array.isArray(r.fields) &&
        r.fields.every((f: unknown) => typeof f === "string") &&
        (r.target_id === null || typeof r.target_id === "string"),
    ) ||
    !v.totals.every(
      (t: any) =>
        typeof t.currency === "string" &&
        [t.source, t.imported, t.skipped, t.payments].every(
          (x) => typeof x === "string" && /^\d+\.\d{2}$/.test(x),
        ),
    )
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return v;
}
async function request(
  base: string,
  company: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/imports${path}`,
    {
      method: body === undefined ? "GET" : "POST",
      credentials: "include",
      headers:
        body === undefined
          ? {}
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: signal ?? AbortSignal.timeout(30000),
    },
  );
  const v = await r.json().catch(() => null);
  if (!r.ok)
    throw new InvoiceCalculationFailure(
      r.status,
      v?.code ?? "unavailable",
      v?.fields ?? [],
    );
  return v;
}
export async function listImports(
  base: string,
  company: string,
  signal?: AbortSignal,
) {
  const v = await request(base, company, "", undefined, signal);
  if (!Array.isArray(v))
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return v.map(parse);
}
export async function getImport(
  base: string,
  company: string,
  id: string,
  signal?: AbortSignal,
) {
  return parse(
    await request(
      base,
      company,
      `/${encodeURIComponent(id)}`,
      undefined,
      signal,
    ),
  );
}
export async function prepareImport(
  base: string,
  company: string,
  body: ImportPrepare,
) {
  return parse(await request(base, company, "", body));
}
export async function applyImport(
  base: string,
  company: string,
  id: string,
  skip: number[],
) {
  return parse(
    await request(base, company, `/${encodeURIComponent(id)}`, { skip }),
  );
}
export async function parseImportTable(
  base: string,
  company: string,
  body: components["schemas"]["ImportTableInput"],
): Promise<ImportTable> {
  const v = await request(base, company, "/table", body);
  if (
    !v ||
    !Array.isArray(v.headers) ||
    !v.headers.every((x: unknown) => typeof x === "string") ||
    !Array.isArray(v.rows) ||
    !v.rows.every(
      (r: any) =>
        Array.isArray(r) &&
        r.length === v.headers.length &&
        r.every((x) => typeof x === "string"),
    )
  )
    throw new InvoiceCalculationFailure(502, "invalid_response");
  return v;
}

export async function prepareMoneyImport(
  base: string,
  company: string,
  input: components["schemas"]["MoneyPrepare"],
): Promise<ImportBatch> {
  return parse(await request(base, company, "/money", input));
}
