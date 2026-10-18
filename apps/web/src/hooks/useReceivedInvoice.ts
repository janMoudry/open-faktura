import { useSearchParams } from "react-router";
import { useEffect, useRef, useState } from "react";
import {
  getReceived,
  createReceived,
  inboxExecute,
  inboxExtract,
  inboxExtractProvider,
  commandReceived,
  previewReceived,
  SalesFailure,
  type ReceivedInvoice,
  type ReceivedData,
  type ReceivedCommand,
  type ReceivedTotals,
} from "@open-faktura/api-client";
import { useWorkspace } from "./useCompanyWorkspace";
import { apiUrl } from "../services/status";
import type { AppKey } from "../locales";
export function emptyReceived(
  kind: "invoice" | "receipt" = "invoice",
): ReceivedData {
  const today = new Intl.DateTimeFormat("sv-SE", {
    timeZone: "Europe/Prague",
  }).format(new Date());
  return {
    kind,
    ...(kind === "receipt" ? { receipt_method: "card" as const } : {}),
    supplier_contact_id: null,
    supplier: {
      name: "",
      registration_number: "",
      vat_number: "",
      address: { street: "", city: "", postal_code: "", country: "CZ" },
      email: "",
      phone: "",
    },
    external_number: "",
    variable_symbol: "",
    bank_account: "",
    issued_on: today,
    received_on: today,
    due_on: today,
    tax_on: null,
    currency: "CZK",
    description: "",
    note: "",
    lines: [{ description: "", rate: "21", net: "", vat: "" }],
    rounding: "0.00",
  };
}
export function useReceivedInvoice(
  company: string,
  id: string,
  kind: "invoice" | "receipt" = "invoice",
) {
  const [params] = useSearchParams();
  const inboxId = params.get("inbox"),
    inboxRevision = Number(params.get("revision"));
  const { onExpired, setPageDirty, navigateAfterSave } = useWorkspace();
  const [document, setDocument] = useState<ReceivedInvoice | null>(null),
    [data, setData] = useState<ReceivedData>(() => emptyReceived(kind)),
    [initial, setInitial] = useState<ReceivedData>(data),
    [loading, setLoading] = useState(id !== "new"),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<AppKey | "">(""),
    [attempt, setAttempt] = useState(0),
    [preview, setPreview] = useState<ReceivedTotals | null>(null);
  const pending = useRef<{ hash: string; key: string } | null>(null);
  const dirty = JSON.stringify(data) !== JSON.stringify(initial);
  useEffect(() => {
    setPageDirty(dirty);
    return () => setPageDirty(false);
  }, [dirty, setPageDirty]);
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else
      setError(
        e instanceof SalesFailure
          ? e.fields.includes("cash_automatic_date")
            ? "cashAutomaticDate"
            : e.fields.includes("cash_link")
              ? "cashLinkedPayment"
              : e.fields.includes("bank_link")
                ? "bankLinkedPayment"
                : e.status === 409
                  ? "receivedConflict"
                  : e.status === 403
                    ? "companyForbidden"
                    : e.status === 422
                      ? "receivedInvalid"
                      : "requestFailed"
          : "requestFailed",
      );
  }
  useEffect(() => {
    if (id === "new") return;
    const c = new AbortController();
    setLoading(true);
    setError("");
    getReceived(apiUrl, company, id, c.signal)
      .then((d) => {
        if ((d.data.kind ?? "invoice") !== kind)
          throw new SalesFailure(404, "not_found");
        if (!c.signal.aborted) {
          setDocument(d);
          setData(d.data);
          setInitial(d.data);
        }
      })
      .catch((e) => {
        if (!c.signal.aborted) failure(e);
      })
      .finally(() => {
        if (!c.signal.aborted) setLoading(false);
      });
    return () => c.abort();
  }, [company, id, kind, attempt]);
  useEffect(() => {
    const c = new AbortController();
    setPreview(null);
    const timer = setTimeout(() => {
      previewReceived(apiUrl, company, data, c.signal)
        .then((v) => {
          if (!c.signal.aborted) setPreview(v);
        })
        .catch(() => {});
    }, 250);
    return () => {
      c.abort();
      clearTimeout(timer);
    };
  }, [company, data]);
  async function command(
    action: Omit<ReceivedCommand, "revision" | "request_id">,
  ) {
    setBusy(true);
    setError("");
    const body = { ...action, revision: document?.revision ?? 1 };
    const hash = JSON.stringify(body);
    if (pending.current?.hash !== hash)
      pending.current = { hash, key: crypto.randomUUID() };
    try {
      const d = document
        ? await commandReceived(apiUrl, company, document.id, {
            ...body,
            request_id: pending.current.key,
          })
        : inboxId
          ? await (async () => {
              const item = await inboxExecute(apiUrl, company, {
                request_id: pending.current!.key,
                id: inboxId,
                revision: inboxRevision,
                action: "convert",
                data,
              });
              if (!item.received_id)
                throw new SalesFailure(502, "invalid_response");
              return getReceived(apiUrl, company, item.received_id);
            })()
          : await createReceived(apiUrl, company, data, pending.current.key);
      setDocument(d);
      setData(d.data);
      setInitial(d.data);
      pending.current = null;
      if (id === "new")
        navigateAfterSave(
          `/companies/${company}/${kind === "receipt" ? "receipts" : "received-invoices"}/${d.id}`,
        );
      return true;
    } catch (e) {
      failure(e);
      return false;
    } finally {
      setBusy(false);
    }
  }
  async function extractInbox(provider = false) {
    if (!inboxId || busy) return;
    setBusy(true);
    setError("");
    try {
      setData(
        await (provider ? inboxExtractProvider : inboxExtract)(
          apiUrl,
          company,
          inboxId,
        ),
      );
    } catch (e) {
      if (e instanceof SalesFailure && e.status === 401) onExpired();
      else setError("inboxExtractFailed");
    } finally {
      setBusy(false);
    }
  }
  return {
    inboxId,
    extractInbox,
    document,
    data,
    setData,
    loading,
    busy,
    error,
    dirty,
    preview,
    command,
    reload: () => setAttempt((v) => v + 1),
  };
}
