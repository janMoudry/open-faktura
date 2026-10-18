import type { components } from "./contact.generated";
export type Contact = components["schemas"]["Contact"];
export type ContactData = components["schemas"]["ContactData"];
export type ContactPage = components["schemas"]["ContactPage"];
export class ContactFailure extends Error {
  constructor(
    public status: number,
    public code: string,
    public fields: string[] = [],
  ) {
    super(code);
  }
}
export function parseContact(value: unknown): Contact {
  if (!value || typeof value !== "object")
    throw new ContactFailure(502, "invalid_response");
  const c = value as Contact;
  const d = c.data;
  const address = (a: unknown) =>
    !!a &&
    typeof a === "object" &&
    ["street", "city", "postal_code", "country"].every(
      (key) => typeof (a as Record<string, unknown>)[key] === "string",
    );
  if (
    typeof c.id !== "string" ||
    typeof c.company_id !== "string" ||
    !Number.isSafeInteger(c.revision) ||
    typeof c.archived !== "boolean" ||
    !(c.anonymized === undefined || typeof c.anonymized === "boolean") ||
    !d ||
    ![
      "name",
      "registration_number",
      "vat_number",
      "email",
      "phone",
      "bank_account",
      "iban",
      "payment_method",
      "note",
    ].every((key) => typeof (d as Record<string, unknown>)[key] === "string") ||
    !address(d.billing_address) ||
    !Array.isArray(d.delivery_addresses) ||
    !d.delivery_addresses.every(
      (a) =>
        typeof a.id === "string" &&
        typeof a.name === "string" &&
        address(a.address),
    ) ||
    !Array.isArray(d.persons) ||
    !d.persons.every((p) =>
      [p.name, p.email, p.phone].every((v) => typeof v === "string"),
    ) ||
    !Array.isArray(d.recipients) ||
    !d.recipients.every((v) => typeof v === "string") ||
    !(
      d.default_delivery_id === null ||
      typeof d.default_delivery_id === "string"
    ) ||
    !(d.maturity_days === null || Number.isInteger(d.maturity_days))
  )
    throw new ContactFailure(502, "invalid_response");
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
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/contacts${path}`,
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
    throw new ContactFailure(
      response.status,
      value?.code ?? "unavailable",
      Array.isArray(value?.fields) ? value.fields : [],
    );
  return value;
}
export async function listContacts(
  base: string,
  company: string,
  search: {
    query: string;
    archived: boolean;
    page: number;
    page_size?: number;
    tag_id?: string;
  },
  signal?: AbortSignal,
): Promise<ContactPage> {
  const params = new URLSearchParams({
    query: search.query,
    tag_id: search.tag_id ?? "",
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
    throw new ContactFailure(502, "invalid_response");
  return { ...value, items: value.items.map(parseContact) };
}
export async function getContact(
  base: string,
  company: string,
  id: string,
  signal?: AbortSignal,
) {
  return parseContact(
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
export async function createContact(
  base: string,
  company: string,
  body: components["schemas"]["CreateContact"],
  signal?: AbortSignal,
) {
  return parseContact(await request(base, company, "", "POST", body, signal));
}
export async function updateContact(
  base: string,
  company: string,
  id: string,
  body: components["schemas"]["UpdateContact"],
  signal?: AbortSignal,
) {
  return parseContact(
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
export async function lookupContactAres(
  base: string,
  company: string,
  ico: string,
  signal?: AbortSignal,
): Promise<components["schemas"]["RegistryContact"]> {
  const v = await request(
    base,
    company,
    `/ares/${encodeURIComponent(ico)}`,
    "GET",
    undefined,
    signal,
  );
  if (
    !v ||
    ![
      v.name,
      v.registration_number,
      v.vat_number,
      v.billing_address?.street,
      v.billing_address?.city,
      v.billing_address?.postal_code,
      v.billing_address?.country,
    ].every((s) => typeof s === "string")
  )
    throw new ContactFailure(502, "invalid_response");
  return v;
}
