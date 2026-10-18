import type { components } from "./sales-trash.generated";
import { SalesFailure } from "./sales";
export type SalesTrashEntry = components["schemas"]["Entry"];
export type SalesTrashPage = components["schemas"]["Page"];
export type SalesTrashCommand = components["schemas"]["Command"];
function valid(v: unknown): v is SalesTrashEntry {
  if (!v || typeof v !== "object") return false;
  const x = v as Partial<SalesTrashEntry>;
  return (
    typeof x.id === "string" &&
    typeof x.title === "string" &&
    typeof x.customer === "string" &&
    (x.kind === "quote" || x.kind === "template") &&
    Number.isSafeInteger(x.revision) &&
    Number(x.revision) > 0 &&
    (x.deleted_at === null || typeof x.deleted_at === "string")
  );
}
async function request(
  base: string,
  c: string,
  command: SalesTrashCommand | undefined,
  page: number,
  signal?: AbortSignal,
): Promise<unknown> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/sales-trash?page=${page}`,
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
  if (!r.ok) throw new SalesFailure(r.status, "sales_trash");
  return r.json();
}
export async function salesTrash(
  base: string,
  c: string,
  page = 1,
  signal?: AbortSignal,
): Promise<SalesTrashPage> {
  const v = await request(base, c, undefined, page, signal);
  if (!v || typeof v !== "object")
    throw new SalesFailure(502, "invalid_response");
  const x = v as Partial<SalesTrashPage>;
  if (
    !Array.isArray(x.items) ||
    !x.items.every(valid) ||
    !Number.isSafeInteger(x.total) ||
    Number(x.total) < 0 ||
    !Number.isSafeInteger(x.page) ||
    Number(x.page) < 1 ||
    x.page_size !== 25
  )
    throw new SalesFailure(502, "invalid_response");
  return x as SalesTrashPage;
}
export async function changeSalesTrash(
  base: string,
  c: string,
  command: SalesTrashCommand,
): Promise<SalesTrashEntry> {
  const v = await request(base, c, command, 1);
  if (!valid(v)) throw new SalesFailure(502, "invalid_response");
  return v;
}
