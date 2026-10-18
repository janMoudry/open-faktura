import { useCallback, useEffect, useState } from "react";
import {
  changeInvoiceTrash,
  invoiceTrash,
  type TrashCommand,
  type TrashPage,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
export function useTrashCommand() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  return {
    busy,
    error,
    execute: async (company: string, command: TrashCommand) => {
      setBusy(true);
      setError(false);
      try {
        await changeInvoiceTrash(apiUrl, company, command);
        return true;
      } catch {
        setError(true);
        return false;
      } finally {
        setBusy(false);
      }
    },
  };
}
export function useDraftTrash(company: string, page: number) {
  const [attempt, setAttempt] = useState(0);
  const [data, setData] = useState<TrashPage | null>(null);
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let active = true;
    const c = new AbortController();
    setData(null);
    setLoading(true);
    setError(false);
    invoiceTrash(apiUrl, company, page, c.signal)
      .then((v) => {
        if (active) setData(v);
      })
      .catch(() => {
        if (active) setError(true);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
      c.abort();
    };
  }, [company, page, attempt]);
  const reload = useCallback(() => setAttempt((v) => v + 1), []);
  return { data, error, loading, reload };
}
