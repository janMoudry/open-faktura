import type { components } from "./company.generated";
export type Company = components["schemas"]["Company"];
export type CompanySettings = components["schemas"]["Settings"];
export class CompanyFailure extends Error {
  constructor(
    public status: number,
    public code: string,
    public fields: string[] = [],
  ) {
    super(code);
  }
}
const stringFields: (keyof CompanySettings)[] = [
  "name",
  "entity_type",
  "registration_number",
  "vat_number",
  "street",
  "city",
  "postal_code",
  "country",
  "email",
  "phone",
  "tax_mode",
  "tax_effective_from",
  "currency",
  "timezone",
  "payment_method",
  "iban",
  "number_prefix",
];
function parseCompany(value: unknown): Company {
  if (!value || typeof value !== "object")
    throw new CompanyFailure(502, "invalid_response");
  const v = value as Record<string, unknown>;
  const s = v.settings as Record<string, unknown> | undefined;
  if (
    typeof v.id !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    !Number.isInteger(v.step) ||
    typeof v.completed !== "boolean" ||
    !["owner", "editor", "viewer"].includes(String(v.role)) ||
    !s ||
    !stringFields.every((k) => typeof s[k] === "string") ||
    !["maturity_days", "number_year", "next_number"].every((k) =>
      Number.isInteger(s[k]),
    ) ||
    !Array.isArray(v.missing_fields) ||
    !v.missing_fields.every((k) => typeof k === "string") ||
    typeof v.number_preview !== "string" ||
    typeof v.can_issue !== "boolean" ||
    !Array.isArray(v.tax_history) ||
    !v.tax_history.every(
      (p) =>
        p &&
        typeof p.mode === "string" &&
        typeof p.effective_from === "string" &&
        Number.isSafeInteger(p.revision),
    )
  )
    throw new CompanyFailure(502, "invalid_response");
  return value as Company;
}
async function request(
  base: string,
  path: string,
  method = "GET",
  body?: unknown,
  signal?: AbortSignal,
) {
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies${path}`,
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
    throw new CompanyFailure(
      response.status,
      typeof value?.code === "string" ? value.code : "unavailable",
      Array.isArray(value?.fields)
        ? value.fields.filter((f: unknown) => typeof f === "string")
        : [],
    );
  return value;
}
export async function listCompanies(
  base: string,
  signal?: AbortSignal,
): Promise<Company[]> {
  const value = await request(base, "", "GET", undefined, signal);
  if (!Array.isArray(value)) throw new CompanyFailure(502, "invalid_response");
  return value.map(parseCompany);
}
export async function getCompany(
  base: string,
  id: string,
  signal?: AbortSignal,
) {
  return parseCompany(
    await request(base, `/${encodeURIComponent(id)}`, "GET", undefined, signal),
  );
}
export async function createCompany(
  base: string,
  body: components["schemas"]["Create"],
) {
  return parseCompany(await request(base, "", "POST", body));
}
export async function updateCompany(
  base: string,
  id: string,
  body: components["schemas"]["Update"],
) {
  return parseCompany(
    await request(base, `/${encodeURIComponent(id)}`, "PUT", body),
  );
}

export async function deleteCompany(
  base: string,
  id: string,
  revision: number,
): Promise<void> {
  await request(base, `/${encodeURIComponent(id)}`, "DELETE", { revision });
}
