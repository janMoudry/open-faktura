import type { components } from "./document-tags.generated";
import { SalesFailure } from "./sales";
export type DocumentTag = components["schemas"]["Tag"];
export type TagTarget = components["schemas"]["Target"];
export type TagCommand = components["schemas"]["Command"];
export async function documentTags(
  base: string,
  company: string,
  input?: TagCommand | TagTarget,
  signal?: AbortSignal,
): Promise<DocumentTag[]> {
  const endpoint = input && !("action" in input) ? "/target" : "";
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/tags${endpoint}`,
    {
      method: input ? "POST" : "GET",
      credentials: "include",
      headers: input
        ? { "Content-Type": "application/json", "X-Faktura-Request": "1" }
        : undefined,
      body: input ? JSON.stringify(input) : undefined,
      signal: signal ?? AbortSignal.timeout(15000),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "document_tags");
  const v: unknown = await r.json();
  if (
    !Array.isArray(v) ||
    v.some(
      (t) =>
        !t ||
        typeof t !== "object" ||
        typeof t.id !== "string" ||
        typeof t.name !== "string" ||
        !Number.isSafeInteger(t.revision) ||
        t.revision < 1 ||
        !Number.isSafeInteger(t.usage_count) ||
        t.usage_count < 0,
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return v as DocumentTag[];
}
