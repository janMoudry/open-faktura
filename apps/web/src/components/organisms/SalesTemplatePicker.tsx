import { useEffect, useState } from "react";
import { Autocomplete } from "@open-faktura/ui";
import {
  listSales,
  SalesFailure,
  type SalesDocument,
} from "@open-faktura/api-client";
import { apiUrl } from "../../services/status";
import { useAppTranslation } from "../../locales";
export function SalesTemplatePicker({
  company,
  disabled,
  onSelect,
  onExpired,
}: {
  company: string;
  disabled: boolean;
  onSelect: (v: SalesDocument) => void;
  onExpired: () => void;
}) {
  const { t } = useAppTranslation();
  const [query, setQuery] = useState("");
  const [refresh, setRefresh] = useState(0);
  const [items, setItems] = useState<SalesDocument[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  useEffect(() => {
    const c = new AbortController();
    setLoading(true);
    setError(false);
    const timer = setTimeout(
      () => {
        listSales(
          apiUrl,
          company,
          { kind: "template", query, page_size: 25 },
          c.signal,
        )
          .then((v) => {
            if (!c.signal.aborted) setItems(v.items);
          })
          .catch((e) => {
            if (!c.signal.aborted) {
              setItems([]);
              setError(true);
              if (e instanceof SalesFailure && e.status === 401) onExpired();
            }
          })
          .finally(() => {
            if (!c.signal.aborted) setLoading(false);
          });
      },
      query ? 200 : 0,
    );
    return () => {
      clearTimeout(timer);
      c.abort();
    };
  }, [company, query, refresh, onExpired]);
  return (
    <Autocomplete
      label={t("salesTemplate")}
      value={query}
      onChange={setQuery}
      onFocus={() => setRefresh((v) => v + 1)}
      loading={loading}
      disabled={disabled}
      error={error ? t("requestFailed") : undefined}
      options={items.map((v) => ({ value: v.id, label: v.data.name }))}
      onSelect={(id) => {
        const v = items.find((v) => v.id === id);
        if (v) {
          onSelect(v);
          setQuery(v.data.name);
        }
      }}
    />
  );
}
