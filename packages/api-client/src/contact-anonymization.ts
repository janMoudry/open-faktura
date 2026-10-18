import type { components } from "./contact-anonymization.generated";
import { ContactFailure, parseContact, type Contact } from "./contact";
export type ContactAnonymizationCommand = components["schemas"]["Command"];
export async function anonymizeContact(
  base: string,
  company: string,
  contact: string,
  command: ContactAnonymizationCommand,
): Promise<Contact> {
  const response = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(company)}/contacts/${encodeURIComponent(contact)}/anonymize`,
    {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: JSON.stringify(command),
      signal: AbortSignal.timeout(15000),
    },
  );
  const value: unknown = await response.json().catch(() => null);
  if (!response.ok)
    throw new ContactFailure(response.status, "contact_anonymization");
  const result = parseContact(value);
  if (!result.anonymized || !result.archived)
    throw new ContactFailure(502, "invalid_response");
  return result;
}
