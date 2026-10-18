import type { components } from "./bank.generated";
import { isDocumentCurrency } from "./invoice";
import { SalesFailure } from "./sales";
export type BankAccount = components["schemas"]["BankAccount"];
export type BankMovement = components["schemas"]["BankMovement"];
export type BankCommand = components["schemas"]["BankCommand"];
export type BankPreview = components["schemas"]["BankPreview"];
export type BankCandidate = components["schemas"]["BankCandidate"];
export type BankPage = components["schemas"]["BankPage"];
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
    `${b.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/bank${path}`,
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
function account(v: BankAccount): BankAccount {
  if (
    !v ||
    typeof v.id !== "string" ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.data?.iban !== "string" ||
    typeof v.data.name !== "string" ||
    !isDocumentCurrency(v.data.currency) ||
    typeof v.archived !== "boolean"
  )
    invalid();
  return v;
}
function foreignSnapshot(
  f: components["schemas"]["BankForeignSettlement"],
): boolean {
  const bankCurrency = f.bank_currency ?? "CZK";
  return (
    isDocumentCurrency(f.currency) &&
    isDocumentCurrency(bankCurrency) &&
    f.currency !== bankCurrency &&
    /^\d+\.\d{2}$/.test(f.document_amount) &&
    /^\d+\.\d{2}$/.test(f.book_value_czk) &&
    /^-?\d+\.\d{2}$/.test(f.difference_czk) &&
    typeof f.document_rate?.rate === "string" &&
    typeof f.document_rate?.units === "string" &&
    (f.bank_value_czk == null
      ? bankCurrency === "CZK"
      : /^\d+\.\d{2}$/.test(f.bank_value_czk)) &&
    (bankCurrency === "CZK"
      ? f.bank_rate == null
      : typeof f.bank_rate?.rate === "string" &&
        typeof f.bank_rate?.units === "string")
  );
}
function movement(v: BankMovement): BankMovement {
  if (
    !v ||
    typeof v.id !== "string" ||
    typeof v.account_id !== "string" ||
    !isDocumentCurrency(v.data?.currency) ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.number !== "string" ||
    !Array.isArray(v.matches) ||
    !/^\d+\.\d{2}$/.test(v.allocated) ||
    !/^\d+\.\d{2}$/.test(v.remaining) ||
    v.matches.some(
      (x) =>
        !/^\d+\.\d{2}$/.test(x.amount) ||
        (x.foreign != null &&
          (!foreignSnapshot(x.foreign) ||
            (x.foreign.bank_currency ?? "CZK") !== v.data.currency)),
    ) ||
    !v.data ||
    !/^\-?\d+\.\d{2}$/.test(v.data.amount) ||
    !/^\d{4}-\d{2}-\d{2}$/.test(v.data.booked_on)
  )
    invalid();
  return v;
}
function preview(v: BankPreview): BankPreview {
  if (
    !v ||
    !Array.isArray(v.rows) ||
    typeof v.valid !== "boolean" ||
    !Number.isSafeInteger(v.new_count) ||
    !Number.isSafeInteger(v.duplicate_count) ||
    v.rows.some(
      (r) =>
        !["new", "duplicate", "conflict", "invalid"].includes(r.status) ||
        !r.data ||
        !Array.isArray(r.fields),
    )
  )
    invalid();
  return v;
}
export async function bankAccounts(
  b: string,
  c: string,
  signal?: AbortSignal,
): Promise<BankAccount[]> {
  const v = await request(b, c, "/accounts", undefined, signal);
  if (!Array.isArray(v)) invalid();
  return v.map(account);
}
export async function bankMovements(
  b: string,
  c: string,
  params: Record<string, string>,
  signal?: AbortSignal,
): Promise<BankPage> {
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
export async function bankGet(
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
export async function bankCandidates(
  b: string,
  c: string,
  id: string,
  query: string,
  signal?: AbortSignal,
): Promise<BankCandidate[]> {
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
export async function bankPreview(
  b: string,
  c: string,
  account_id: string,
  csv: string,
  signal?: AbortSignal,
) {
  return preview(await request(b, c, "/preview", { account_id, csv }, signal));
}
export async function bankExecute(
  b: string,
  c: string,
  cmd: BankCommand,
): Promise<components["schemas"]["BankReply"]> {
  const v = await request(b, c, "/commands", cmd);
  if (v?.kind === "account") return { ...v, value: account(v.value) };
  if (v?.kind === "movement") return { ...v, value: movement(v.value) };
  if (v?.kind === "import") return { ...v, value: preview(v.value) };
  if (v?.kind === "batch") {
    if (!Array.isArray(v.value) || v.value.length > 25) invalid();
    return { ...v, value: v.value.map(movement) };
  }
  return invalid();
}

export type BankStatement = components["schemas"]["BankStatement"];
export async function bankStatementPreview(
  b: string,
  c: string,
  file: BankStatement,
) {
  return preview(await request(b, c, "/statement-preview", file));
}
export async function bankSuggestions(
  b: string,
  c: string,
  id: string,
  signal?: AbortSignal,
): Promise<BankCandidate[]> {
  const v = await request(
    b,
    c,
    `/movements/${encodeURIComponent(id)}/suggestions`,
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

export type BankBatchPreview = components["schemas"]["BankBatchPreview"];
export async function bankBatchPreview(
  b: string,
  c: string,
  movement_ids: string[],
  signal?: AbortSignal,
): Promise<BankBatchPreview> {
  const v = await request(b, c, "/batch-preview", { movement_ids }, signal);
  const statuses = [
    "ready",
    "matched",
    "archived",
    "no_symbol",
    "no_match",
    "ambiguous",
    "partial",
    "competing_movement",
    "shared_target",
    "too_many",
  ];
  if (
    !v ||
    !Array.isArray(v.items) ||
    v.items.length < 1 ||
    v.items.length > 25 ||
    !Number.isSafeInteger(v.ready_count) ||
    typeof v.fingerprint !== "string" ||
    !/^[0-9a-f]{64}$/.test(v.fingerprint)
  )
    invalid();
  for (const item of v.items) {
    if (!item) invalid();
    movement(item.movement);
    if (!statuses.includes(item.status)) invalid();
    const x = item.candidate;
    if (
      item.status === "ready" &&
      (!x ||
        typeof x.document_id !== "string" ||
        typeof x.number !== "string" ||
        typeof x.partner !== "string" ||
        typeof x.outstanding !== "string" ||
        !/^\d+\.\d{2}$/.test(x.outstanding) ||
        (x.payment_id !== null && typeof x.payment_id !== "string") ||
        !["invoice", "receipt", "advance", "credit"].includes(
          x.document_kind,
        ) ||
        !Number.isSafeInteger(x.revision) ||
        !["invoice", "received", "advance", "credit"].includes(x.target_kind))
    )
      invalid();
    if (item.status !== "ready" && x !== null) invalid();
  }
  if (
    v.ready_count !==
    v.items.filter((i: { status: string }) => i.status === "ready").length
  )
    invalid();
  return v;
}

export type BankBalance = components["schemas"]["BankBalance"];
export async function bankBalance(
  b: string,
  c: string,
  params: Record<string, string>,
  signal?: AbortSignal,
): Promise<BankBalance> {
  const v = await request(
    b,
    c,
    `/balance?${new URLSearchParams(params)}`,
    undefined,
    signal,
  );
  if (
    !v ||
    !isDocumentCurrency(v.currency) ||
    !Number.isSafeInteger(v.count) ||
    ![v.opening_balance, v.opening, v.closing].every(
      (x) => x === null || (typeof x === "string" && /^-?\d+\.\d{2}$/.test(x)),
    ) ||
    ![v.income, v.expense].every(
      (x) => typeof x === "string" && /^\d+\.\d{2}$/.test(x),
    ) ||
    !(
      v.opening_on === null ||
      (typeof v.opening_on === "string" &&
        /^\d{4}-\d{2}-\d{2}$/.test(v.opening_on))
    )
  )
    invalid();
  return v;
}

export type BankConnectionStatus =
  components["schemas"]["BankConnectionStatus"];
export type BankConnectionCommand =
  components["schemas"]["BankConnectionCommand"];
function connectionStatus(v: BankConnectionStatus): BankConnectionStatus {
  if (
    !v ||
    !Number.isSafeInteger(v.revision) ||
    typeof v.configured !== "boolean" ||
    typeof v.busy !== "boolean" ||
    (v.provider !== null && typeof v.provider !== "string") ||
    (v.last_sync_at !== null && typeof v.last_sync_at !== "string") ||
    !Array.isArray(v.providers) ||
    v.providers.some(
      (p) => typeof p.id !== "string" || typeof p.label !== "string",
    )
  )
    invalid();
  if (
    typeof v.schedule_enabled !== "boolean" ||
    ![15, 60, 360, 1440].includes(v.interval_minutes) ||
    (v.next_run_at !== null && typeof v.next_run_at !== "string") ||
    (v.schedule_error !== null && typeof v.schedule_error !== "string")
  )
    invalid();
  if (
    typeof v.matching_enabled !== "boolean" ||
    typeof v.matching_treasury !== "boolean" ||
    (v.matching_next_at !== null && typeof v.matching_next_at !== "string") ||
    (v.matching_error !== null && typeof v.matching_error !== "string") ||
    (v.matching_last !== null &&
      (!v.matching_last ||
        typeof v.matching_last.at !== "string" ||
        !Number.isSafeInteger(v.matching_last.checked) ||
        !Number.isSafeInteger(v.matching_last.matched) ||
        v.matching_last.matched < 0 ||
        v.matching_last.checked > 25 ||
        v.matching_last.checked < v.matching_last.matched))
  )
    invalid();
  const r = v.last_run;
  if (
    r !== null &&
    (!r ||
      !["save", "disconnect", "test", "sync", "schedule", "matching"].includes(
        r.kind,
      ) ||
      !["running", "success", "failed"].includes(r.status) ||
      typeof r.started_at !== "string" ||
      (r.finished_at !== null && typeof r.finished_at !== "string") ||
      (r.error_code !== null && typeof r.error_code !== "string") ||
      !Number.isSafeInteger(r.imported) ||
      !Number.isSafeInteger(r.duplicates))
  )
    invalid();
  return v;
}
export async function bankConnectionStatus(
  b: string,
  c: string,
  account: string,
  signal?: AbortSignal,
): Promise<BankConnectionStatus> {
  return connectionStatus(
    await request(
      b,
      c,
      `/connection?account_id=${encodeURIComponent(account)}`,
      undefined,
      signal,
    ),
  );
}
export async function bankConnectionExecute(
  b: string,
  c: string,
  cmd: BankConnectionCommand,
): Promise<BankConnectionStatus> {
  return connectionStatus(await request(b, c, "/connection", cmd));
}

export async function bankSplitCandidates(
  b: string,
  c: string,
  id: string,
  query: string,
  signal?: AbortSignal,
): Promise<BankCandidate[]> {
  const v = await request(
    b,
    c,
    `/movements/${encodeURIComponent(id)}/split-candidates?${new URLSearchParams({ query })}`,
    undefined,
    signal,
  );
  if (
    !Array.isArray(v) ||
    v.some(
      (x) =>
        !x ||
        typeof x.document_id !== "string" ||
        typeof x.number !== "string" ||
        typeof x.partner !== "string" ||
        !["invoice", "received", "advance", "credit"].includes(x.target_kind) ||
        !Number.isSafeInteger(x.revision) ||
        (x.payment_id !== null && typeof x.payment_id !== "string") ||
        !/^\d+\.\d{2}$/.test(x.available),
    )
  )
    invalid();
  return v;
}

export type BankForeignCandidate =
  components["schemas"]["BankForeignCandidate"];
export async function bankForeignCandidates(
  b: string,
  c: string,
  id: string,
  query: string,
  signal?: AbortSignal,
): Promise<BankForeignCandidate[]> {
  const v = await request(
    b,
    c,
    `/movements/${encodeURIComponent(id)}/foreign-candidates?${new URLSearchParams({ query })}`,
    undefined,
    signal,
  );
  if (
    !Array.isArray(v) ||
    v.some(
      (x) =>
        !x ||
        typeof x.document_id !== "string" ||
        (x.payment_id !== null && typeof x.payment_id !== "string") ||
        (x.payment_id === null
          ? x.payment_amount !== null
          : typeof x.payment_amount !== "string" ||
            !/^\d+\.\d{2}$/.test(x.payment_amount)) ||
        typeof x.number !== "string" ||
        typeof x.partner !== "string" ||
        !["invoice", "received"].includes(x.target_kind) ||
        !Number.isSafeInteger(x.revision) ||
        !isDocumentCurrency(x.currency) ||
        !/^\d+\.\d{2}$/.test(x.outstanding),
    )
  )
    invalid();
  return v;
}

export type BankForeignReport = components["schemas"]["BankForeignReport"];
export async function bankForeignReport(
  b: string,
  c: string,
  query: Record<string, string>,
  signal?: AbortSignal,
): Promise<BankForeignReport> {
  const v = await request(
    b,
    c,
    `/foreign-report?${new URLSearchParams(query)}`,
    undefined,
    signal,
  );
  const money = (s: unknown) =>
    typeof s === "string" && /^-?\d+\.\d{2}$/.test(s);
  if (
    !v ||
    v.currency !== "CZK" ||
    !money(v.gains) ||
    !money(v.losses) ||
    !money(v.net) ||
    !Number.isSafeInteger(v.total) ||
    !Number.isSafeInteger(v.page) ||
    !Array.isArray(v.items) ||
    v.items.some(
      (x: BankForeignReport["items"][number]) =>
        !x ||
        typeof x.id !== "string" ||
        typeof x.movement_id !== "string" ||
        typeof x.document_id !== "string" ||
        !["invoice", "received"].includes(x.target_kind) ||
        !money(x.bank_amount) ||
        !x.settlement ||
        !money(x.settlement.difference_czk) ||
        !money(x.settlement.document_amount) ||
        !money(x.settlement.book_value_czk) ||
        !foreignSnapshot(x.settlement),
    )
  )
    invalid();
  return v;
}
