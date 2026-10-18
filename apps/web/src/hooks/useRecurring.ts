import { useEffect, useRef, useState } from "react";
import {
  createRecurring,
  commandRecurring,
  getRecurring,
  getDraft,
  previewRecurring,
  SalesFailure,
  type RecurringRule,
  type RecurringData,
  type RecurringCommand,
  type Company,
} from "@open-faktura/api-client";
import { useWorkspace } from "./useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { newDraft } from "../utils/invoice";
import { apiUrl } from "../services/status";
import { salesError } from "../utils/sales";
export function useRecurring(company: Company, id: string, source: string) {
  const { language } = useAppTranslation();
  const { onExpired, setPageDirty, navigateAfterSave } = useWorkspace();
  const initial = useRef<RecurringData>({
    rate_policy: "stored",
    name: "",
    schedule: {
      first_on: newDraft(company, language).issued_on!,
      frequency: "month",
      interval: 1,
      month_end: false,
      end_on: null,
      count: null,
      timezone: company.settings.timezone,
    },
    maturity_days: company.settings.maturity_days,
    auto_issue: false,
    mail: null,
    invoice: newDraft(company, language),
  });
  const [record, setRecord] = useState<RecurringRule | null>(null),
    [data, setData] = useState(initial.current),
    [error, setError] = useState<AppKey | "">(""),
    [busy, setBusy] = useState(false),
    [loading, setLoading] = useState(id !== "new" || !!source),
    [attempt, setAttempt] = useState(0),
    [dates, setDates] = useState<string[]>([]),
    [previewError, setPreviewError] = useState(false),
    [previewLoading, setPreviewLoading] = useState(true);
  const pending = useRef<{ hash: string; key: string } | null>(null);
  const dirty =
    JSON.stringify(data) !== JSON.stringify(record?.data ?? initial.current);
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else
      setError(
        e instanceof SalesFailure && e.status === 422
          ? "recInvalid"
          : salesError(e),
      );
  }
  useEffect(() => {
    setPageDirty(dirty);
    return () => setPageDirty(false);
  }, [dirty, setPageDirty]);
  useEffect(() => {
    const c = new AbortController();
    if (id === "new") {
      if (!source) {
        setLoading(false);
        return;
      }
      getDraft(apiUrl, company.id, source, c.signal)
        .then((v) => {
          if (c.signal.aborted) return;
          if (v.data.calculation.calculation_version !== "of-czk-v1")
            throw new SalesFailure(422, "invalid");
          setData({
            ...initial.current,
            name: v.data.calculation.lines[0]?.description ?? "",
            invoice: {
              ...v.data,
              payment: { ...v.data.payment, variable_symbol: "" },
            },
          });
        })
        .catch((e) => {
          if (!c.signal.aborted) failure(e);
        })
        .finally(() => {
          if (!c.signal.aborted) setLoading(false);
        });
    } else {
      setLoading(true);
      getRecurring(apiUrl, company.id, id, c.signal)
        .then((v) => {
          if (!c.signal.aborted) {
            setRecord(v);
            setData(v.data);
            setError("");
          }
        })
        .catch((e) => {
          if (!c.signal.aborted) failure(e);
        })
        .finally(() => {
          if (!c.signal.aborted) setLoading(false);
        });
    }
    return () => c.abort();
  }, [company.id, id, source, attempt]);
  useEffect(() => {
    const c = new AbortController();
    setDates([]);
    setPreviewError(false);
    setPreviewLoading(true);
    const timer = setTimeout(() => {
      previewRecurring(
        apiUrl,
        company.id,
        data.schedule,
        record?.generated ?? 0,
        c.signal,
      )
        .then((v) => {
          if (!c.signal.aborted) {
            setDates(v);
            setPreviewLoading(false);
          }
        })
        .catch(() => {
          if (!c.signal.aborted) {
            setPreviewError(true);
            setPreviewLoading(false);
          }
        });
    }, 250);
    return () => {
      clearTimeout(timer);
      c.abort();
    };
  }, [company.id, data.schedule, record?.generated]);
  useEffect(() => {
    if (!record || record.status !== "active" || dirty) return;
    const controller = new AbortController();
    const timer = setInterval(() => {
      getRecurring(apiUrl, company.id, id, controller.signal)
        .then((v) => {
          if (!controller.signal.aborted) {
            setRecord(v);
            setData(v.data);
          }
        })
        .catch((e) => {
          if (!controller.signal.aborted) failure(e);
        });
    }, 5000);
    return () => {
      clearInterval(timer);
      controller.abort();
    };
  }, [company.id, id, record?.status, dirty]);
  async function command(
    action: Omit<RecurringCommand, "request_id" | "revision">,
  ) {
    setBusy(true);
    setError("");
    try {
      const body =
        id === "new" ? { data } : { ...action, revision: record!.revision };
      const hash = JSON.stringify(body);
      if (pending.current?.hash !== hash)
        pending.current = { hash, key: crypto.randomUUID() };
      const v =
        id === "new"
          ? await createRecurring(apiUrl, company.id, data, pending.current.key)
          : await commandRecurring(apiUrl, company.id, id, {
              ...action,
              revision: record!.revision,
              request_id: pending.current.key,
            });
      setRecord(v);
      setData(v.data);
      pending.current = null;
      if (id === "new")
        navigateAfterSave(`/companies/${company.id}/recurring/${v.id}`);
      return true;
    } catch (e) {
      failure(e);
      return false;
    } finally {
      setBusy(false);
    }
  }
  return {
    record,
    data,
    setData,
    error,
    busy,
    loading,
    dirty,
    dates,
    previewError,
    previewLoading,
    command,
    reload: () => setAttempt((v) => v + 1),
    onExpired,
  };
}
