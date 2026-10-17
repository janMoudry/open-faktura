import type { components } from "./export.generated";
import { SalesFailure } from "./sales";
export type ExportRequest = components["schemas"]["ExportRequest"];
export async function exportRegister(
  base: string,
  company: string,
  input: ExportRequest,
  signal?: AbortSignal,
) {
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/exports`,
    {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: JSON.stringify(input),
      signal: signal
        ? AbortSignal.any([signal, AbortSignal.timeout(60000)])
        : AbortSignal.timeout(60000),
    },
  );
  if (!response.ok) {
    const v = await response.json().catch(() => null);
    throw new SalesFailure(
      response.status,
      v?.code ?? "unavailable",
      v?.fields ?? [],
    );
  }
  return response.blob();
}
