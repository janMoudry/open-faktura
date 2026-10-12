import { type Dispatch, type SetStateAction } from "react";
import type { CompanySettings, ContactData } from "@open-faktura/api-client";
import { emptyContact } from "../utils/contact";
import { useAresLookup } from "./useAresLookup";
export function companyRegistryData(settings: CompanySettings): ContactData {
  return {
    ...emptyContact(),
    name: settings.name,
    registration_number: settings.registration_number,
    vat_number: settings.vat_number,
    billing_address: {
      street: settings.street,
      city: settings.city,
      postal_code: settings.postal_code,
      country: settings.country,
    },
  };
}
export function useCompanyAres(
  company: string,
  setSettings: Dispatch<SetStateAction<CompanySettings>>,
  onExpired: () => void,
) {
  const update: Dispatch<SetStateAction<ContactData>> = (action) =>
    setSettings((current) => {
      const original = companyRegistryData(current);
      const next = typeof action === "function" ? action(original) : action;
      return {
        ...current,
        name: next.name,
        vat_number: next.vat_number,
        ...next.billing_address,
        country: current.country,
      };
    });
  return useAresLookup(company, update, onExpired);
}
