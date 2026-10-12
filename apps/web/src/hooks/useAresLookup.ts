import {
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import {
  ContactFailure,
  lookupContactAres,
  type ContactData,
} from "@open-faktura/api-client";
import type { AppKey } from "../locales";
import { apiUrl } from "../services/status";
export function useAresLookup(
  company: string,
  setData: Dispatch<SetStateAction<ContactData>>,
  onExpired: () => void,
) {
  const pending = useRef<AbortController | null>(null);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<AppKey | "">("");
  const cancel = () => {
    pending.current?.abort();
    pending.current = null;
    setLoading(false);
    setMessage("");
  };
  useEffect(() => {
    cancel();
    return () => {
      pending.current?.abort();
    };
  }, [company]);
  async function lookup(data: ContactData) {
    if (pending.current && !pending.current.signal.aborted) return;
    cancel();
    const ico = data.registration_number.trim();
    if (!ico) return;
    if (!/^\d{8}$/.test(ico)) {
      setMessage("aresInvalid");
      return;
    }
    const controller = new AbortController();
    pending.current = controller;
    setLoading(true);
    setMessage("aresLoading");
    try {
      const result = await lookupContactAres(
        apiUrl,
        company,
        ico,
        controller.signal,
      );
      if (controller.signal.aborted) return;
      if (
        data.billing_address.country &&
        result.billing_address.country &&
        data.billing_address.country !== result.billing_address.country
      ) {
        setMessage("aresCountry");
        return;
      }
      setData((current) => {
        if (current.registration_number.trim() !== ico) return current;
        const address = { ...current.billing_address };
        if (
          result.billing_address.country &&
          address.country &&
          result.billing_address.country !== address.country
        )
          return current;
        for (const key of [
          "street",
          "city",
          "postal_code",
          "country",
        ] as const) {
          if (!address[key].trim()) address[key] = result.billing_address[key];
        }
        return {
          ...current,
          name: current.name.trim() ? current.name : result.name,
          vat_number: current.vat_number.trim()
            ? current.vat_number
            : result.vat_number,
          billing_address: address,
        };
      });
      setMessage("aresFilled");
    } catch (error) {
      if (controller.signal.aborted) return;
      if (error instanceof ContactFailure && error.status === 401) onExpired();
      setMessage(
        error instanceof ContactFailure && error.status === 404
          ? "aresMissing"
          : error instanceof ContactFailure && error.status === 422
            ? "aresInvalid"
            : "aresUnavailable",
      );
    } finally {
      if (!controller.signal.aborted) {
        pending.current = null;
        setLoading(false);
      }
    }
  }
  return { loading, message, lookup, cancel };
}
