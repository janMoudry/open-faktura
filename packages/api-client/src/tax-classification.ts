import type { components } from "./tax-classification.generated";
import { SalesFailure } from "./sales";
export type TaxEvidenceSource = components["schemas"]["Source"];
export type TaxEvidenceData = components["schemas"]["Data"];
export type TaxEvidenceCommand = components["schemas"]["Command"];
export type TaxEvidenceSummary = components["schemas"]["Summary"];
async function request(
  base: string,
  c: string,
  path: string,
  input?: TaxEvidenceCommand,
): Promise<unknown> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/reports/tax/classifications${path}`,
    {
      credentials: "include",
      signal: AbortSignal.timeout(30000),
      ...(input
        ? {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "X-Faktura-Request": "1",
            },
            body: JSON.stringify(input),
          }
        : {}),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "unavailable");
  return r.json();
}
export async function taxEvidenceSource(
  base: string,
  c: string,
  kind: string,
  id: string,
): Promise<TaxEvidenceSource> {
  const v = (await request(
    base,
    c,
    `/${encodeURIComponent(kind)}/${encodeURIComponent(id)}`,
  )) as TaxEvidenceSource;
  if (
    !v ||
    !validObservation(v.observation) ||
    (v.classification !== null && !validClassification(v.classification))
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function setTaxEvidence(
  base: string,
  c: string,
  kind: string,
  id: string,
  input: TaxEvidenceCommand,
): Promise<components["schemas"]["Classification"]> {
  const v = await request(
    base,
    c,
    `/${encodeURIComponent(kind)}/${encodeURIComponent(id)}`,
    input,
  );
  if (!validClassification(v)) throw new SalesFailure(502, "invalid_response");
  return v as components["schemas"]["Classification"];
}
export async function taxEvidenceSummary(
  base: string,
  c: string,
  year: number,
): Promise<TaxEvidenceSummary> {
  const v = (await request(
    base,
    c,
    `?${new URLSearchParams({ year: String(year) })}`,
  )) as TaxEvidenceSummary;
  if (
    !v ||
    v.year !== year ||
    ![
      v.unclassified_turnover,
      v.unclassified_input,
      v.outdated,
      v.classified,
    ].every((x) => Number.isSafeInteger(x) && x >= 0) ||
    ![v.included_turnover_czk, v.claimed_input_vat_czk].every(
      (x) => typeof x === "string" && /^-?\d+(\.\d{1,2})?$/.test(x),
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}

function validObservation(value: unknown): boolean {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  const money = (x: unknown) =>
    x === null || (typeof x === "string" && /^-?\d+(\.\d{1,2})?$/.test(x));
  const date = (x: unknown) =>
    x === null || (typeof x === "string" && /^\d{4}-\d{2}-\d{2}$/.test(x));
  return (
    typeof v.kind === "string" &&
    [
      "invoice",
      "credit_note",
      "tax_receipt",
      "retail",
      "received",
      "purchase_receipt",
      "cash",
    ].includes(v.kind) &&
    typeof v.id === "string" &&
    /^\d+$/.test(v.id) &&
    typeof v.number === "string" &&
    ["sales", "purchases"].includes(String(v.side)) &&
    typeof v.fingerprint === "string" &&
    /^[a-f0-9]{64}$/.test(v.fingerprint) &&
    money(v.recorded_vat_czk) &&
    money(v.turnover_net_czk) &&
    date(v.vat_on) &&
    date(v.turnover_on)
  );
}
function validClassification(value: unknown): boolean {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  const d = v.data as Record<string, unknown> | null;
  return (
    Number.isSafeInteger(v.revision) &&
    Number(v.revision) > 0 &&
    typeof v.current === "boolean" &&
    validObservation(v.observation) &&
    !!d &&
    typeof d.reason === "string" &&
    (d.turnover_included === null ||
      typeof d.turnover_included === "boolean") &&
    (d.claimed_input_vat_czk === null ||
      (typeof d.claimed_input_vat_czk === "string" &&
        /^\d+(\.\d{1,2})?$/.test(d.claimed_input_vat_czk))) &&
    (d.claim_on === null ||
      (typeof d.claim_on === "string" &&
        /^\d{4}-\d{2}-\d{2}$/.test(d.claim_on)))
  );
}
