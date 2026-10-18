import type { components } from "./document-output.generated";
import { SalesFailure } from "./sales";
export type DocumentOutputRequest = components["schemas"]["Request"];
export async function documentOutput(
  base: string,
  company: string,
  request: DocumentOutputRequest,
): Promise<Blob> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/document-output`,
    {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: JSON.stringify(request),
      signal: AbortSignal.timeout(120000),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "document_output");
  if (
    !["application/pdf", "application/rtf"].some((t) =>
      r.headers.get("Content-Type")?.includes(t),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return r.blob();
}
