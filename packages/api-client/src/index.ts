export type Health = { status: "ok" | "ready" | "unavailable" };
export async function getHealth(
  baseUrl: string,
  endpoint: "health" | "ready",
  signal?: AbortSignal,
): Promise<Health> {
  const response = await fetch(
    `${baseUrl.replace(/\/$/, "")}/api/${endpoint}`,
    { signal },
  );
  const data: unknown = await response.json();
  if (
    !data ||
    typeof data !== "object" ||
    !("status" in data) ||
    !["ok", "ready", "unavailable"].includes(String(data.status))
  )
    throw new Error("Neplatná odpověď serveru");
  if (
    !response.ok &&
    !(response.status === 503 && data.status === "unavailable")
  )
    throw new Error("Server vrátil chybu");
  return data as Health;
}

export * from "./auth";
export {
  CompanyFailure,
  listCompanies,
  getCompany,
  createCompany,
  updateCompany,
  deleteCompany,
  type Company,
  type CompanySettings,
} from "./company";
export * from "./contact";
export * from "./invoice";
export * from "./draft";
export * from "./payment";
export * from "./import";

export * from "./catalog";

export * from "./sales";
export * from "./public-quote";

export * from "./invoice-delivery";

export * from "./smtp";

export * from "./attachment";

export * from "./export";

export * from "./advance";

export * from "./tax-receipt";

export * from "./advance-settlement";

export * from "./credit-note";

export * from "./recurring";

export * from "./communication";

export * from "./invoice-share";
export * from "./public-invoice";

export * from "./received";

export * from "./bank";

export * from "./cash";

export * from "./retail";

export * from "./inbox";
export * from "./stock";
export * from "./preferences";
export * from "./period-report";
export * from "./exchange-rate";

export * from "./company-admin";

export * from "./isdoc";

export * from "./vat-registry";

export * from "./document-tags";

export * from "./bank-notification";

export * from "./document-output";

export * from "./dashboard-widgets";

export * from "./document-style";

export * from "./inbox-mailbox";

export * from "./vies";

export * from "./document-exports";

export * from "./draft-trash";

export * from "./tax-classification";

export * from "./taxpayer-filing-profile";

export * from "./contact-anonymization";

export * from "./inbox-notification";

export * from "./inbox-delivery";

export * from "./sales-trash";
