import type { components } from "./document-exports.generated";
import { SalesFailure } from "./sales";
export type ExportTarget = components["schemas"]["Target"];
export type ExportVersion = components["schemas"]["Version"];
export type ExportHistory = components["schemas"]["History"];
export type DocumentExportRequest = components["schemas"]["Request"];
export type DocumentExportUnlock = components["schemas"]["Unlock"];
async function call(
  base: string,
  company: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/document-exports${path}`,
    {
      method: body ? "POST" : "GET",
      credentials: "include",
      headers: body
        ? { "Content-Type": "application/json", "X-Faktura-Request": "1" }
        : undefined,
      body: body ? JSON.stringify(body) : undefined,
      signal: signal ?? AbortSignal.timeout(30000),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "document_export");
  return r;
}
function validate(v: unknown): ExportHistory {
  if (!v || typeof v !== "object")
    throw new SalesFailure(502, "invalid_response");
  const h = v as Partial<ExportHistory>;
  if (
    !(h.locked_by_export === null || typeof h.locked_by_export === "string") ||
    !Array.isArray(h.exports) ||
    h.exports.some(
      (e) =>
        !e ||
        typeof e.id !== "string" ||
        typeof e.sha256 !== "string" ||
        typeof e.actor_id !== "string" ||
        typeof e.created_at !== "string" ||
        !Array.isArray(e.targets) ||
        e.targets.some(
          (t) =>
            !t ||
            !["invoice", "sales", "received"].includes(t.kind) ||
            typeof t.id !== "string" ||
            !Number.isSafeInteger(t.revision),
        ),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return h as ExportHistory;
}
export async function documentExportHistory(
  base: string,
  company: string,
  target: ExportTarget,
  signal?: AbortSignal,
) {
  return validate(
    await (
      await call(
        base,
        company,
        `?kind=${target.kind}&id=${encodeURIComponent(target.id)}`,
        undefined,
        signal,
      )
    ).json(),
  );
}
export async function documentExportCreate(
  base: string,
  company: string,
  request: DocumentExportRequest,
) {
  return (await call(base, company, "", request)).blob();
}
export async function documentExportUnlock(
  base: string,
  company: string,
  command: DocumentExportUnlock,
) {
  return validate(await (await call(base, company, "/unlock", command)).json());
}
export async function documentExportFile(
  base: string,
  company: string,
  id: string,
) {
  return (await call(base, company, `/${encodeURIComponent(id)}`)).blob();
}
