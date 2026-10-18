import { SalesFailure } from "./sales";
import type { components } from "./company-admin.generated";
export type AdministrationSnapshot = components["schemas"]["Snapshot"];
export type AdministrationAction =
  | { action: "add_member"; email: string; role: "editor" | "viewer" }
  | { action: "set_role"; user_id: string; role: "editor" | "viewer" }
  | { action: "transfer" | "remove_member"; user_id: string }
  | { action: "leave" | "restore" };
function validate(v: unknown): AdministrationSnapshot {
  if (!v || typeof v !== "object")
    throw new SalesFailure(502, "invalid_response");
  const d = v as Partial<AdministrationSnapshot>;
  if (
    typeof d.company_id !== "string" ||
    !Number.isSafeInteger(d.revision) ||
    Number(d.revision) < 1 ||
    typeof d.deleted !== "boolean" ||
    !Array.isArray(d.members) ||
    d.members.some(
      (m) =>
        !m ||
        typeof m.user_id !== "string" ||
        typeof m.email !== "string" ||
        typeof m.name !== "string" ||
        !["owner", "editor", "viewer"].includes(m.role),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return d as AdministrationSnapshot;
}
export async function companyAdministration(
  base: string,
  company: string,
  command?: components["schemas"]["Command"],
  signal?: AbortSignal,
): Promise<AdministrationSnapshot> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/administration`,
    {
      method: command ? "POST" : "GET",
      credentials: "include",
      headers: command
        ? { "Content-Type": "application/json", "X-Faktura-Request": "1" }
        : undefined,
      body: command ? JSON.stringify(command) : undefined,
      signal: signal ?? AbortSignal.timeout(15000),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "company_administration");
  return validate(await r.json());
}
export async function companyTrash(
  base: string,
  signal?: AbortSignal,
): Promise<AdministrationSnapshot[]> {
  const r = await fetch(`${base.replace(/\/$/, "")}/api/companies/trash`, {
    credentials: "include",
    signal: signal ?? AbortSignal.timeout(15000),
  });
  if (!r.ok) throw new SalesFailure(r.status, "company_administration");
  const value: unknown = await r.json();
  if (!Array.isArray(value)) throw new SalesFailure(502, "invalid_response");
  return value.map(validate);
}
export async function companyArchive(
  base: string,
  company: string,
): Promise<Blob> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/archive`,
    { credentials: "include", signal: AbortSignal.timeout(120000) },
  );
  if (!r.ok) throw new SalesFailure(r.status, "company_archive");
  if (!r.headers.get("Content-Type")?.includes("application/zip"))
    throw new SalesFailure(502, "invalid_response");
  return r.blob();
}
