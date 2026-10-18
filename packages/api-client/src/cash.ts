import type { components } from "./cash.generated";
import { SalesFailure } from "./sales";
export type CashAccount = components["schemas"]["CashAccount"];
export type CashMovement = components["schemas"]["CashMovement"];
export type CashCommand = components["schemas"]["CashCommand"];
export type CashCandidate = components["schemas"]["CashCandidate"];
export type CashPage = components["schemas"]["CashPage"];
function invalid(): never {
  throw new SalesFailure(502, "invalid_response");
}
async function request(
  b: string,
  c: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const r = await fetch(
    `${b.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/cash${path}`,
    {
      method: body === undefined ? "GET" : "POST",
      credentials: "include",
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
function account(v: CashAccount): CashAccount {
  if (
    !v ||
    typeof v.id !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.data?.opening_on !== "string" ||
    !/^-?\d+\.\d{2}$/.test(v.data.opening_balance) ||
    typeof v.data.name !== "string" ||
    v.data.currency !== "CZK" ||
    typeof v.archived !== "boolean"
  )
    invalid();
  return v;
}
function movement(v: CashMovement): CashMovement {
  if (
    !v ||
    typeof v.id !== "string" ||
    typeof v.account_id !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.number !== "string" ||
    typeof v.cancelled !== "boolean" ||
    typeof v.automatic !== "boolean" ||
    typeof v.data?.person !== "string" ||
    !Array.isArray(v.matches) ||
    !v.data ||
    !/^\-?\d+\.\d{2}$/.test(v.data.amount) ||
    !/^\d{4}-\d{2}-\d{2}$/.test(v.data.booked_on)
  )
    invalid();
  if (v.tax !== null) taxSnapshot(v.tax);
  return v;
}
export async function cashAccounts(
  b: string,
  c: string,
  signal?: AbortSignal,
): Promise<CashAccount[]> {
  const v = await request(b, c, "/accounts", undefined, signal);
  if (!Array.isArray(v)) invalid();
  return v.map(account);
}
export async function cashMovements(
  b: string,
  c: string,
  params: Record<string, string>,
  signal?: AbortSignal,
): Promise<CashPage> {
  const v = await request(
    b,
    c,
    `/movements?${new URLSearchParams(params)}`,
    undefined,
    signal,
  );
  if (
    !Array.isArray(v?.items) ||
    !Number.isSafeInteger(v.total) ||
    !Number.isSafeInteger(v.page)
  )
    invalid();
  return { ...v, items: v.items.map(movement) };
}
export async function cashGet(
  b: string,
  c: string,
  id: string,
  signal?: AbortSignal,
) {
  return movement(
    await request(
      b,
      c,
      `/movements/${encodeURIComponent(id)}`,
      undefined,
      signal,
    ),
  );
}
export async function cashCandidates(
  b: string,
  c: string,
  id: string,
  query: string,
  signal?: AbortSignal,
): Promise<CashCandidate[]> {
  const v = await request(
    b,
    c,
    `/movements/${encodeURIComponent(id)}/candidates?${new URLSearchParams({ query })}`,
    undefined,
    signal,
  );
  if (
    !Array.isArray(v) ||
    v.some(
      (x) =>
        !x ||
        typeof x.document_id !== "string" ||
        !Number.isSafeInteger(x.revision) ||
        !["invoice", "received", "advance", "credit"].includes(x.target_kind),
    )
  )
    invalid();
  return v;
}
export async function cashExecute(
  b: string,
  c: string,
  cmd: CashCommand,
): Promise<components["schemas"]["CashReply"]> {
  const v = await request(b, c, "/commands", cmd);
  if (v?.kind === "terminal") return { ...v, value: terminal(v.value) };
  if (v?.kind === "settings") return { ...v, value: settings(v.value) };
  if (v?.kind === "account") return { ...v, value: account(v.value) };
  if (v?.kind === "movement") return { ...v, value: movement(v.value) };
  return invalid();
}

export type CashBalance = components["schemas"]["CashBalance"];
export async function cashBalance(
  b: string,
  c: string,
  params: Record<string, string>,
  signal?: AbortSignal,
): Promise<CashBalance> {
  const v = await request(
    b,
    c,
    `/balance?${new URLSearchParams(params)}`,
    undefined,
    signal,
  );
  if (
    !v ||
    !Number.isSafeInteger(v.count) ||
    [v.opening, v.income, v.expense, v.closing].some(
      (x) => typeof x !== "string" || !/^-?\d+\.\d{2}$/.test(x),
    )
  )
    invalid();
  return v;
}

export type CashSettings = components["schemas"]["CashSettings"];
function settings(v: CashSettings): CashSettings {
  if (
    !v ||
    !Number.isSafeInteger(v.revision) ||
    (v.account_id !== null && typeof v.account_id !== "string")
  )
    invalid();
  return v;
}
export async function cashSettings(
  b: string,
  c: string,
  signal?: AbortSignal,
): Promise<CashSettings> {
  return settings(await request(b, c, "/settings", undefined, signal));
}
export async function cashPdf(
  b: string,
  c: string,
  id: string,
  language: "cs" | "en",
): Promise<Blob> {
  const r = await fetch(
    `${b.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/cash/movements/${encodeURIComponent(id)}/pdf?language=${language}`,
    { credentials: "include", signal: AbortSignal.timeout(30000) },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  if (!r.headers.get("content-type")?.startsWith("application/pdf")) invalid();
  return r.blob();
}

export type CashTaxInput = components["schemas"]["CashTaxInput"];
export type CashTaxSnapshot = components["schemas"]["CashTaxSnapshot"];
function taxSnapshot(v: CashTaxSnapshot): CashTaxSnapshot {
  if (
    !v ||
    typeof v.input_amount !== "string" ||
    !["net", "gross"].includes(v.tax?.price_mode) ||
    !["0", "12", "21"].includes(v.tax.vat_percent) ||
    !/^\d{4}-\d{2}-\d{2}$/.test(v.tax.tax_on) ||
    typeof v.calculation_version !== "string" ||
    !v.amounts ||
    [v.amounts.net, v.amounts.vat, v.amounts.gross].some(
      (x) => typeof x !== "string" || !/^\d+\.\d{2}$/.test(x),
    )
  )
    invalid();
  return v;
}
export async function cashTaxPreview(
  b: string,
  c: string,
  amount: string,
  tax: CashTaxInput,
  signal?: AbortSignal,
) {
  return taxSnapshot(
    await request(b, c, "/tax-preview", { amount, tax }, signal),
  );
}

export type CashTerminal = components["schemas"]["CashTerminal"];
function terminal(v: CashTerminal): CashTerminal {
  if (
    !v ||
    typeof v.id !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.code !== "string" ||
    typeof v.name !== "string" ||
    typeof v.archived !== "boolean"
  )
    invalid();
  return v;
}
export async function cashTerminals(
  b: string,
  c: string,
  signal?: AbortSignal,
): Promise<CashTerminal[]> {
  const v = await request(b, c, "/terminals", undefined, signal);
  if (!Array.isArray(v)) invalid();
  return v.map(terminal);
}
