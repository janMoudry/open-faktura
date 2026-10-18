import type { components } from "./draft-trash.generated";
import { SalesFailure } from "./sales";
export type TrashEntry = components["schemas"]["Entry"];
export type TrashPage = components["schemas"]["Page"];
export type TrashCommand = components["schemas"]["Command"];
function valid(v: unknown): v is TrashEntry {
  if (!v || typeof v !== "object") return false;
  const x = v as Partial<TrashEntry>;
  return (
    typeof x.id === "string" &&
    typeof x.title === "string" &&
    typeof x.customer === "string" &&
    Number.isSafeInteger(x.revision) &&
    (x.deleted_at === null || typeof x.deleted_at === "string")
  );
}
async function request(
  base: string,
  c: string,
  command: TrashCommand | undefined,
  page: number,
  signal?: AbortSignal,
): Promise<unknown> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/invoice-trash?page=${page}`,
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
  if (!r.ok) throw new SalesFailure(r.status, "draft_trash");
  return r.json();
}
export async function invoiceTrash(
  base: string,
  c: string,
  page = 1,
  signal?: AbortSignal,
): Promise<TrashPage> {
  const v = await request(base, c, undefined, page, signal);
  if (!v || typeof v !== "object")
    throw new SalesFailure(502, "invalid_response");
  const x = v as Partial<TrashPage>;
  if (
    !Array.isArray(x.items) ||
    !x.items.every(valid) ||
    !Number.isSafeInteger(x.total) ||
    !Number.isSafeInteger(x.page) ||
    x.page_size !== 25
  )
    throw new SalesFailure(502, "invalid_response");
  return x as TrashPage;
}
export async function changeInvoiceTrash(
  base: string,
  c: string,
  command: TrashCommand,
): Promise<TrashEntry> {
  const v = await request(base, c, command, 1);
  if (!valid(v)) throw new SalesFailure(502, "invalid_response");
  return v;
}
