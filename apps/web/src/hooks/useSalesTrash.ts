import { useCallback, useEffect, useState } from "react";
import {
  SalesFailure,
  changeSalesTrash,
  salesTrash,
  type SalesTrashCommand,
  type SalesTrashPage,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
import { useWorkspace } from "./useCompanyWorkspace";
export function useSalesTrashCommand() {
  const { onExpired } = useWorkspace();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  return {
    busy,
    error,
    execute: async (company: string, command: SalesTrashCommand) => {
      setBusy(true);
      setError(false);
      try {
        await changeSalesTrash(apiUrl, company, command);
        return true;
      } catch (e) {
        if (e instanceof SalesFailure && e.status === 401) onExpired();
        else setError(true);
        return false;
      } finally {
        setBusy(false);
      }
    },
  };
}
export function useSalesTrash(company: string, page: number) {
  const { onExpired } = useWorkspace();
  const [attempt, setAttempt] = useState(0);
  const [data, setData] = useState<SalesTrashPage | null>(null);
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let active = true;
    const c = new AbortController();
    setData(null);
    setLoading(true);
    setError(false);
    salesTrash(apiUrl, company, page, c.signal)
      .then((v) => {
        if (active) setData(v);
      })
      .catch((e: unknown) => {
        if (!active) return;
        if (e instanceof SalesFailure && e.status === 401) onExpired();
        else setError(true);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
      c.abort();
    };
  }, [company, page, attempt, onExpired]);
  const reload = useCallback(() => setAttempt((v) => v + 1), []);
  return { data, error, loading, reload };
}
