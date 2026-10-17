import type { components } from "./period-report.generated";
import { SalesFailure } from "./sales";
export type PeriodReport = components["schemas"]["PeriodReport"];
export async function periodReport(
  base: string,
  company: string,
  query: Record<string, string>,
  signal?: AbortSignal,
): Promise<PeriodReport> {
  const r = await fetch(
    base.replace(/\/$/, "") +
      "/api/companies/" +
      encodeURIComponent(company) +
      "/reports/period?" +
      new URLSearchParams(query),
    { credentials: "include", signal: signal ?? AbortSignal.timeout(30000) },
  );
  if (!r.ok) throw new SalesFailure(r.status, "unavailable");
  const v = await r.json();
  if (
    typeof v?.sales !== "string" ||
    typeof v.purchases !== "string" ||
    !Array.isArray(v.months) ||
    !Array.isArray(v.items) ||
    !Number.isSafeInteger(v.total)
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
