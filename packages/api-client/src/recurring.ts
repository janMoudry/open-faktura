import type { components } from "./recurring.generated";
import { SalesFailure } from "./sales";
export type RecurringRule = components["schemas"]["RecurringRule"];
export type RecurringData = components["schemas"]["RecurringData"];
export type RecurringCommand = components["schemas"]["RecurringCommand"];
export type RecurringPage = components["schemas"]["RecurringPage"];
async function request(
  base: string,
  c: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/recurring${path}`,
    {
      credentials: "include",
      method: body === undefined ? "GET" : "POST",
      headers:
        body === undefined
          ? undefined
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: signal ?? AbortSignal.timeout(30000),
    },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  return r.json();
}
function rule(v: unknown): RecurringRule {
  const d = v as RecurringRule;
  if (
    !d ||
    typeof d.id !== "string" ||
    typeof d.company_id !== "string" ||
    !Number.isSafeInteger(d.revision) ||
    !Number.isSafeInteger(d.generated) ||
    !["paused", "active", "completed", "error"].includes(d.status) ||
    typeof d.data?.name !== "string" ||
    !d.data?.schedule ||
    !d.data?.invoice?.supplier ||
    !Array.isArray(d.data.invoice.calculation?.lines) ||
    !Array.isArray(d.next_dates) ||
    !d.next_dates.every(
      (x) => typeof x === "string" && /^\d{4}-\d{2}-\d{2}$/.test(x),
    ) ||
    !Array.isArray(d.occurrences)
  )
    throw new SalesFailure(502, "invalid_response");
  return d;
}
export async function createRecurring(
  b: string,
  c: string,
  data: RecurringData,
  key: string,
) {
  return rule(await request(b, c, "", { data, request_id: key }));
}
export async function commandRecurring(
  b: string,
  c: string,
  id: string,
  cmd: RecurringCommand,
) {
  return rule(await request(b, c, `/${encodeURIComponent(id)}`, cmd));
}
export async function getRecurring(
  b: string,
  c: string,
  id: string,
  signal?: AbortSignal,
) {
  return rule(
    await request(b, c, `/${encodeURIComponent(id)}`, undefined, signal),
  );
}
export async function listRecurring(
  b: string,
  c: string,
  query: string,
  page: number,
  signal?: AbortSignal,
): Promise<RecurringPage> {
  const v = await request(
    b,
    c,
    `?${new URLSearchParams({ query, page: String(page) })}`,
    undefined,
    signal,
  );
  if (!Array.isArray(v?.items) || !Number.isSafeInteger(v.total))
    throw new SalesFailure(502, "invalid_response");
  return { ...v, items: v.items.map(rule) };
}
export async function previewRecurring(
  b: string,
  c: string,
  schedule: RecurringData["schedule"],
  index: number,
  signal?: AbortSignal,
): Promise<string[]> {
  const v = await request(b, c, "/preview", { schedule, index }, signal);
  if (
    !Array.isArray(v) ||
    !v.every((x) => typeof x === "string" && /^\d{4}-\d{2}-\d{2}$/.test(x))
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
