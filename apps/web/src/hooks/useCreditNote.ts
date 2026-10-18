import { useEffect, useRef, useState } from "react";
import {
  getCredit,
  creditCommand,
  SalesFailure,
  type CreditNote,
  type CreditData,
  type CreditCommand,
} from "@open-faktura/api-client";
import { useWorkspace } from "./useCompanyWorkspace";
import { apiUrl } from "../services/status";
import type { AppKey } from "../locales";
import { salesError } from "../utils/sales";
export function useCreditNote(companyId: string, id: string) {
  const { onExpired, setPageDirty } = useWorkspace();
  const [document, setDocument] = useState<CreditNote | null>(null),
    [data, setData] = useState<CreditData | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [busy, setBusy] = useState(false),
    [attempt, setAttempt] = useState(0);
  const pending = useRef<{ hash: string; key: string } | null>(null);
  const dirty =
    !!document &&
    !!data &&
    JSON.stringify(data) !== JSON.stringify(document.data);
  useEffect(() => {
    setPageDirty(dirty);
    return () => setPageDirty(false);
  }, [dirty, setPageDirty]);
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else if (
      e instanceof SalesFailure &&
      e.fields.some((f) => f.startsWith("credit_"))
    )
      setError(
        e.fields.includes("credit_offset")
          ? "creditOffsetError"
          : e.fields.includes("credit_dates_reason")
            ? "creditDatesError"
            : "creditInvalid",
      );
    else setError(salesError(e));
  }
  useEffect(() => {
    const c = new AbortController();
    setDocument(null);
    setData(null);
    setError("");
    getCredit(apiUrl, companyId, id, c.signal)
      .then((v) => {
        if (!c.signal.aborted) {
          setDocument(v);
          setData(v.data);
        }
      })
      .catch((e) => {
        if (!c.signal.aborted) failure(e);
      });
    return () => c.abort();
  }, [companyId, id, attempt]);
  async function command(
    action: Omit<CreditCommand, "request_id" | "revision">,
  ) {
    if (!document) return false;
    setBusy(true);
    setError("");
    try {
      const body = { ...action, revision: document.revision };
      const hash = JSON.stringify(body);
      if (pending.current?.hash !== hash)
        pending.current = { hash, key: crypto.randomUUID() };
      const v = await creditCommand(apiUrl, companyId, id, {
        ...body,
        request_id: pending.current.key,
      });
      setDocument(v);
      setData(v.data);
      pending.current = null;
      return true;
    } catch (e) {
      failure(e);
      return false;
    } finally {
      setBusy(false);
    }
  }
  return {
    document,
    data,
    setData,
    error,
    busy,
    dirty,
    command,
    failure,
    reload: () => setAttempt((v) => v + 1),
  };
}
