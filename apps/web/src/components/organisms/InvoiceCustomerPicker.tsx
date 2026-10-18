import { useEffect, useState } from "react";
import { Autocomplete } from "@open-faktura/ui";
import {
  listContacts,
  ContactFailure,
  type Contact,
} from "@open-faktura/api-client";
import { apiUrl } from "../../services/status";
import { useAppTranslation } from "../../locales";
export function InvoiceCustomerPicker({
  label,
  company,
  disabled,
  onSelect,
  onExpired,
}: {
  label?: string;
  company: string;
  disabled: boolean;
  onSelect: (c: Contact) => void;
  onExpired: () => void;
}) {
  const { t } = useAppTranslation();
  const [query, setQuery] = useState("");
  const [refresh, setRefresh] = useState(0);
  const [items, setItems] = useState<Contact[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(false);
    const timer = setTimeout(
      () => {
        listContacts(
          apiUrl,
          company,
          { query, archived: false, page: 1 },
          controller.signal,
        )
          .then((v) => {
            if (!controller.signal.aborted) setItems(v.items);
          })
          .catch((e) => {
            if (!controller.signal.aborted) {
              setError(true);
              if (e instanceof ContactFailure && e.status === 401) onExpired();
            }
          })
          .finally(() => {
            if (!controller.signal.aborted) setLoading(false);
          });
      },
      query ? 200 : 0,
    );
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [company, query, refresh, onExpired]);
  return (
    <Autocomplete
      label={label ?? t("draftFindCustomer")}
      value={query}
      onChange={setQuery}
      onFocus={() => setRefresh((v) => v + 1)}
      disabled={disabled}
      loading={loading}
      error={error ? t("requestFailed") : undefined}
      options={items.map((c) => ({
        value: c.id,
        label: c.data.name,
        description: c.data.registration_number,
      }))}
      onSelect={(id) => {
        const c = items.find((c) => c.id === id);
        if (c) onSelect(c);
      }}
    />
  );
}
