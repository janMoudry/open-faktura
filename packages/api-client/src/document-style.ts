import type { components } from "./document-style.generated";
import { SalesFailure } from "./sales";
export type DocumentStyle = components["schemas"]["Style"];
export type StyleSnapshot = components["schemas"]["Snapshot"];
export type StyleCommand = components["schemas"]["Command"];
export async function documentStyle(
  base: string,
  company: string,
  command?: StyleCommand,
  signal?: AbortSignal,
): Promise<StyleSnapshot> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/document-style`,
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
  if (!r.ok) throw new SalesFailure(r.status, "document_style");
  const value: unknown = await r.json();
  if (!value || typeof value !== "object")
    throw new SalesFailure(502, "invalid_response");
  const v = value as Partial<StyleSnapshot>;
  const d = v.data;
  if (
    !Number.isSafeInteger(v.revision) ||
    Number(v.revision) < 0 ||
    !d ||
    [d.before_items_cs, d.before_items_en].some(
      (text) =>
        text !== undefined &&
        (typeof text !== "string" || [...text].length > 1000),
    ) ||
    typeof d.footer_cs !== "string" ||
    typeof d.footer_en !== "string" ||
    !(
      d.delivery_note_prices === undefined ||
      typeof d.delivery_note_prices === "boolean"
    ) ||
    !(d.logo_base64 === null || typeof d.logo_base64 === "string") ||
    !(d.signature_base64 === null || typeof d.signature_base64 === "string")
  )
    throw new SalesFailure(502, "invalid_response");
  return {
    ...v,
    data: {
      ...d,
      delivery_note_prices: d.delivery_note_prices ?? false,
      before_items_cs: d.before_items_cs ?? "",
      before_items_en: d.before_items_en ?? "",
    },
  } as StyleSnapshot;
}
