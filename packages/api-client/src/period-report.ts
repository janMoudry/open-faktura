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

export type Dashboard = components["schemas"]["Dashboard"];
export async function dashboardReport(
  base: string,
  company: string,
  query: Record<string, string>,
  signal?: AbortSignal,
): Promise<Dashboard> {
  const r = await fetch(
    base.replace(/\/$/, "") +
      "/api/companies/" +
      encodeURIComponent(company) +
      "/reports/dashboard?" +
      new URLSearchParams(query),
    { credentials: "include", signal: signal ?? AbortSignal.timeout(30000) },
  );
  if (!r.ok) throw new SalesFailure(r.status, "unavailable");
  return r.json();
}

export type TaxReport = components["schemas"]["TaxReport"];
export async function taxReport(
  base: string,
  company: string,
  query: Record<string, string>,
  signal?: AbortSignal,
): Promise<TaxReport> {
  const r = await fetch(
    base.replace(/\/$/, "") +
      "/api/companies/" +
      encodeURIComponent(company) +
      "/reports/tax?" +
      new URLSearchParams(query),
    { credentials: "include", signal: signal ?? AbortSignal.timeout(30000) },
  );
  if (!r.ok) throw new SalesFailure(r.status, "unavailable");
  return r.json();
}
