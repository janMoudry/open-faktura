import { useEffect, useRef, useState } from "react";
import { Autocomplete } from "@open-faktura/ui";
import {
  CatalogFailure,
  catalogLine,
  listCatalogs,
  type Catalog,
  type DraftData,
} from "@open-faktura/api-client";
import { apiUrl } from "../../services/status";
import { useAppTranslation } from "../../locales";
import { moneyText } from "../../utils/invoice";
export function CatalogLinePicker({
  company,
  disabled,
  priceMode,
  taxMode,
  onSelect,
  onExpired,
}: {
  company: string;
  disabled: boolean;
  priceMode: DraftData["calculation"]["price_mode"];
  taxMode: DraftData["calculation"]["tax_mode"];
  onSelect: (line: DraftData["calculation"]["lines"][number]) => void;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [query, setQuery] = useState("");
  const [refresh, setRefresh] = useState(0);
  const [items, setItems] = useState<Catalog[]>([]);
  const [loading, setLoading] = useState(false);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState(false);
  const selection = useRef<AbortController | null>(null);
  const latestSelect = useRef(onSelect);
  latestSelect.current = onSelect;
  useEffect(() => {
    selection.current?.abort();
    setAdding(false);
    return () => selection.current?.abort();
  }, [company, priceMode, taxMode, disabled]);
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(false);
    const timer = setTimeout(
      () => {
        listCatalogs(
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
              if (e instanceof CatalogFailure && e.status === 401) onExpired();
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
  async function select(id: string) {
    if (disabled || adding) return;
    selection.current?.abort();
    const controller = new AbortController();
    selection.current = controller;
    setAdding(true);
    setError(false);
    try {
      const line = await catalogLine(
        apiUrl,
        company,
        id,
        { price_mode: priceMode, tax_mode: taxMode },
        controller.signal,
      );
      if (!controller.signal.aborted) {
        latestSelect.current(line);
        setQuery("");
      }
    } catch (e) {
      if (!controller.signal.aborted) {
        setError(true);
        if (e instanceof CatalogFailure && e.status === 401) onExpired();
      }
    } finally {
      if (!controller.signal.aborted) setAdding(false);
    }
  }
  return (
    <Autocomplete
      label={t("catalogPick")}
      value={query}
      onFocus={() => setRefresh((v) => v + 1)}
      onChange={setQuery}
      disabled={disabled || adding}
      loading={loading || adding}
      error={error ? t("catalogLineFailed") : undefined}
      options={items.map((item) => ({
        value: item.id,
        label: item.data.name,
        description: `${item.data.code} · ${moneyText(item.data.unit_price, language)}`,
      }))}
      onSelect={(id) => void select(id)}
    />
  );
}
