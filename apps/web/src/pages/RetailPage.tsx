import { useEffect, useRef, useState } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router";
import {
  Alert,
  Badge,
  Button,
  DatePicker,
  Field,
  Heading,
  Select,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  retailList,
  retailHistory,
  type RetailAudit,
  retailGet,
  retailExecute,
  retailPreview,
  retailPdfUrl,
  cashTerminals,
  calculateInvoice,
  SalesFailure,
  type RetailData,
  type RetailDocument,
  type RetailPage as Page,
  type InvoiceTotals,
  type CashTerminal,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import { newDraft, moneyText, contactParty } from "../utils/invoice";
import { InvoiceLines } from "../components/organisms/InvoiceLines";
import { InvoiceCustomerPicker } from "../components/organisms/InvoiceCustomerPicker";
import { RetailPostings } from "../components/organisms/RetailPostings";
function failure(e: unknown): AppKey {
  return e instanceof SalesFailure && e.status === 409
    ? "retailConflict"
    : e instanceof SalesFailure && e.status === 422
      ? "retailInvalid"
      : "cashFailed";
}
export function RetailPage() {
  const { companyId = "", retailId } = useParams();
  return retailId ? (
    <RetailEditor
      key={`${companyId}:${retailId}`}
      companyId={companyId}
      id={retailId}
    />
  ) : (
    <RetailRegister key={companyId} companyId={companyId} />
  );
}
function RetailRegister({ companyId }: { companyId: string }) {
  const { t, language } = useAppTranslation(),
    { company, onExpired } = useWorkspace();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "",
    cancelled = params.get("cancelled") === "true",
    page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1));
  const [value, setValue] = useState<Page | null>(null),
    [input, setInput] = useState(query),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0),
    [selected, setSelected] = useState<string[]>([]);
  useEffect(() => {
    const c = new AbortController();
    setValue(null);
    setSelected([]);
    setError(false);
    retailList(apiUrl, companyId, query, cancelled, page, c.signal)
      .then(setValue)
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, query, cancelled, page, attempt]);
  return (
    <div className="space-y-group">
      <div className="flex flex-wrap items-center justify-between gap-field">
        <Heading level={1} size="page">
          {t("retailTitle")}
        </Heading>
        {company?.role !== "viewer" && (
          <Link
            className="ui-button ui-button-primary ui-button-md"
            to={`/companies/${companyId}/retail/new`}
          >
            {t("retailNew")}
          </Link>
        )}
      </div>
      <section className="workspace-panel">
        <form
          className="flex flex-wrap items-end gap-field p-field"
          onSubmit={(e) => {
            e.preventDefault();
            setParams({ q: input, cancelled: String(cancelled) });
          }}
        >
          <Field
            label={t("retailSearch")}
            value={input}
            onChange={(e) => setInput(e.target.value)}
          />
          <Button type="submit" variant="secondary">
            {t("retailSearchButton")}
          </Button>
          <Select
            label={t("terminalStatus")}
            value={String(cancelled)}
            onChange={(v) => setParams({ q: query, cancelled: v })}
            options={[
              { value: "false", label: t("retailActive") },
              { value: "true", label: t("retailCancelled") },
            ]}
          />
        </form>
        {error ? (
          <Alert tone="danger">
            {t("cashFailed")}{" "}
            <Button onClick={() => setAttempt((v) => v + 1)}>
              {t("cashRetry")}
            </Button>
          </Alert>
        ) : !value ? (
          <p className="p-field" role="status">
            {t("cashLoading")}
          </p>
        ) : (
          <>
            <div className="overflow-x-auto">
              <Table className="whitespace-nowrap">
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("retailSelect")}</TableHead>
                    <TableHead>{t("retailNumber")}</TableHead>
                    <TableHead>{t("draftIssued")}</TableHead>
                    <TableHead>{t("retailCustomer")}</TableHead>
                    <TableHead>{t("terminalTitle")}</TableHead>
                    <TableHead>{t("retailTotal")}</TableHead>
                  </TableRow>
                </TableHeader>
                <tbody>
                  {value.items.map((v) => (
                    <TableRow key={v.id}>
                      <TableCell>
                        <input
                          type="checkbox"
                          aria-label={t("retailSelect") + " " + v.number}
                          disabled={
                            company?.role === "viewer" ||
                            v.cancelled ||
                            !!v.posting_id
                          }
                          checked={selected.includes(v.id)}
                          onChange={(e) =>
                            setSelected((old) =>
                              e.target.checked
                                ? [...old, v.id]
                                : old.filter((id) => id !== v.id),
                            )
                          }
                        />
                      </TableCell>
                      <TableCell>
                        <Link to={`/companies/${companyId}/retail/${v.id}`}>
                          {v.number}
                        </Link>{" "}
                        {v.posting_id && <Badge>{t("retailPosted")}</Badge>}
                      </TableCell>
                      <TableCell>
                        {v.data.invoice.issued_on
                          ? new Date(
                              v.data.invoice.issued_on + "T12:00:00",
                            ).toLocaleDateString(language)
                          : "—"}
                      </TableCell>
                      <TableCell>
                        {v.data.invoice.customer?.name ?? t("retailAnonymous")}
                      </TableCell>
                      <TableCell>{v.terminal?.name ?? "—"}</TableCell>
                      <TableCell>
                        {moneyText(v.totals.payable, language)}
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!value.items.length && (
              <p className="p-field">{t("retailEmpty")}</p>
            )}
            <div className="flex justify-end gap-control p-field">
              <Button
                variant="secondary"
                disabled={page === 1}
                onClick={() =>
                  setParams({
                    q: query,
                    cancelled: String(cancelled),
                    page: String(page - 1),
                  })
                }
              >
                {t("retailPrevious")}
              </Button>
              <Button
                variant="secondary"
                disabled={page * 25 >= value.total}
                onClick={() =>
                  setParams({
                    q: query,
                    cancelled: String(cancelled),
                    page: String(page + 1),
                  })
                }
              >
                {t("retailNext")}
              </Button>
            </div>
          </>
        )}
      </section>
      <RetailPostings
        companyId={companyId}
        selected={selected}
        canWrite={company?.role !== "viewer"}
        onChanged={() => {
          setSelected([]);
          setAttempt((v) => v + 1);
        }}
      />
    </div>
  );
}
function RetailEditor({ companyId, id }: { companyId: string; id: string }) {
  const { company, onExpired } = useWorkspace(),
    { t, language } = useAppTranslation(),
    navigate = useNavigate();
  const [data, setData] = useState<RetailData | null>(
      company && id === "new"
        ? {
            invoice: newDraft(company, language),
            terminal_id: null,
            payments: { cash: "0", card: "0", voucher: "0" },
            internal_note: "",
          }
        : null,
    ),
    [doc, setDoc] = useState<RetailDocument | null>(null),
    [editing, setEditing] = useState(id === "new"),
    [totals, setTotals] = useState<InvoiceTotals | null>(null),
    [valid, setValid] = useState(false),
    [terminals, setTerminals] = useState<CashTerminal[]>([]),
    [error, setError] = useState<AppKey | "">(""),
    [busy, setBusy] = useState(false),
    [reason, setReason] = useState(""),
    [history, setHistory] = useState<RetailAudit[]>([]);
  const pending = useRef<{ input: string; key: string } | null>(null);
  const canWrite = company?.role !== "viewer";
  useEffect(() => {
    const c = new AbortController();
    Promise.all([
      cashTerminals(apiUrl, companyId, c.signal),
      id === "new"
        ? Promise.resolve(null)
        : retailGet(apiUrl, companyId, id, c.signal),
    ])
      .then(([ts, d]) => {
        if (!c.signal.aborted) {
          setTerminals(ts);
          if (d) {
            setDoc(d);
            setData(d.data);
            setTotals(d.totals);
          }
        }
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(failure(e));
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, id]);
  useEffect(() => {
    if (!data || !editing) return;
    const c = new AbortController();
    setValid(false);
    const timer = setTimeout(() => {
      calculateInvoice(
        apiUrl,
        companyId,
        { ...data.invoice.calculation, calculation_version: "of-czk-v1" },
        c.signal,
      )
        .then((v) => {
          if (!c.signal.aborted) setTotals(v);
        })
        .catch(() => {
          if (!c.signal.aborted) setTotals(null);
        });
      retailPreview(apiUrl, companyId, data, c.signal)
        .then(() => {
          if (!c.signal.aborted) setValid(true);
        })
        .catch(() => {});
    }, 200);
    return () => {
      clearTimeout(timer);
      c.abort();
    };
  }, [data, editing, companyId]);
  useEffect(() => {
    if (id === "new") return;
    const c = new AbortController();
    retailHistory(apiUrl, companyId, id, c.signal)
      .then(setHistory)
      .catch((e) => {
        if (!c.signal.aborted) setError(failure(e));
      });
    return () => c.abort();
  }, [companyId, id, doc?.revision]);
  async function save(cancel = false) {
    if (!data || busy) return;
    setBusy(true);
    setError("");
    const action =
      cancel && doc
        ? {
            action: "cancel" as const,
            id: doc.id,
            revision: doc.revision,
            reason,
          }
        : {
            action: "save" as const,
            id: doc?.id ?? null,
            revision: doc?.revision ?? null,
            data,
            reason: reason || t("retailNew"),
          };
    const input = JSON.stringify(action);
    if (pending.current?.input !== input)
      pending.current = { input, key: crypto.randomUUID() };
    try {
      const v = await retailExecute(apiUrl, companyId, {
        ...action,
        request_id: pending.current.key,
      });
      pending.current = null;
      setDoc(v);
      setData(v.data);
      setEditing(false);
      setReason("");
      navigate(`/companies/${companyId}/retail/${v.id}`, { replace: true });
    } catch (e) {
      setError(failure(e));
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  const disabled =
    !editing || !canWrite || busy || !!doc?.cancelled || !!doc?.posting_id;
  return (
    <div className="space-y-group">
      <div className="flex flex-wrap items-center justify-between gap-field">
        <Heading level={1} size="page">
          {doc?.number ?? t("retailNew")}
        </Heading>
        <Link to={`/companies/${companyId}/retail`}>{t("retailBack")}</Link>
      </div>
      {error && <Alert tone="danger">{t(error)}</Alert>}
      {!data ? (
        <p role="status">{t("cashLoading")}</p>
      ) : (
        <>
          {doc?.posting_id && <Badge>{t("retailPosted")}</Badge>}
          {doc?.cancelled && <Badge>{t("retailCancelled")}</Badge>}
          <div className="workspace-panel">
            <div className="p-field grid gap-field sm:grid-cols-2">
              <DatePicker
                label={t("draftIssued")}
                value={data.invoice.issued_on ?? ""}
                disabled={disabled}
                onChange={(v) =>
                  setData({
                    ...data,
                    invoice: { ...data.invoice, issued_on: v || null },
                  })
                }
              />
              <Select
                label={t("terminalTitle")}
                value={data.terminal_id ?? ""}
                disabled={disabled}
                onChange={(v) => setData({ ...data, terminal_id: v || null })}
                options={[
                  { value: "", label: t("retailNoTerminal") },
                  ...terminals
                    .filter((v) => !v.archived || v.id === data.terminal_id)
                    .map((v) => ({ value: v.id, label: v.name })),
                ]}
              />
              <div>
                <p className="font-medium">
                  {data.invoice.customer?.name ?? t("retailAnonymous")}
                </p>
                {!disabled && (
                  <>
                    <InvoiceCustomerPicker
                      label={t("retailCustomer")}
                      company={companyId}
                      disabled={disabled}
                      onExpired={onExpired}
                      onSelect={(c) =>
                        setData({
                          ...data,
                          invoice: {
                            ...data.invoice,
                            customer: contactParty(c),
                            contact_id: c.id,
                          },
                        })
                      }
                    />
                    {data.invoice.customer && (
                      <Button
                        variant="ghost"
                        onClick={() =>
                          setData({
                            ...data,
                            invoice: {
                              ...data.invoice,
                              customer: null,
                              contact_id: null,
                            },
                          })
                        }
                      >
                        {t("retailClearCustomer")}
                      </Button>
                    )}
                  </>
                )}
              </div>
            </div>
            <InvoiceLines
              value={data.invoice.calculation}
              onChange={(v) =>
                setData({
                  ...data,
                  invoice: { ...data.invoice, calculation: v },
                })
              }
              disabled={disabled}
              errors={[]}
              totals={totals}
            />
            <div className="p-field space-y-field">
              <Heading level={2} size="card">
                {t("retailPayments")}
              </Heading>
              <p>{t("retailPaymentHelp")}</p>
              <p className="font-semibold">
                {t("retailTotal")}:{" "}
                {totals ? moneyText(totals.payable, language) : "—"}
              </p>
              <div className="grid gap-field sm:grid-cols-3">
                {(["cash", "card", "voucher"] as const).map((method) => (
                  <Field
                    key={method}
                    label={t(
                      (
                        {
                          cash: "retailCash",
                          card: "retailCard",
                          voucher: "retailVoucher",
                        } as const
                      )[method],
                    )}
                    inputMode="decimal"
                    disabled={disabled}
                    value={data.payments[method]}
                    onChange={(e) =>
                      setData({
                        ...data,
                        payments: {
                          ...data.payments,
                          [method]: e.target.value.replace(",", "."),
                        },
                      })
                    }
                  />
                ))}
              </div>
              {!disabled && totals && (
                <Button
                  variant="secondary"
                  onClick={() =>
                    setData({
                      ...data,
                      payments: {
                        cash: totals.payable,
                        card: "0",
                        voucher: "0",
                      },
                    })
                  }
                >
                  {t("retailAllCash")}
                </Button>
              )}
              <Field
                label={t("retailNote")}
                disabled={disabled}
                value={data.invoice.note}
                onChange={(e) =>
                  setData({
                    ...data,
                    invoice: { ...data.invoice, note: e.target.value },
                  })
                }
              />
              <Field
                label={t("retailInternalNote")}
                disabled={disabled}
                value={data.internal_note}
                onChange={(e) =>
                  setData({ ...data, internal_note: e.target.value })
                }
              />
              <Select
                label={t("retailLanguage")}
                value={data.invoice.language}
                disabled={disabled}
                onChange={(v) =>
                  setData({
                    ...data,
                    invoice: { ...data.invoice, language: v as "cs" | "en" },
                  })
                }
                options={[
                  { value: "cs", label: "Čeština" },
                  { value: "en", label: "English" },
                ]}
              />
            </div>
          </div>
          <div className="flex flex-wrap items-end gap-field">
            {doc && (
              <a
                className="ui-button ui-button-secondary ui-button-md"
                target="_blank"
                rel="noreferrer"
                href={retailPdfUrl(apiUrl, companyId, doc.id)}
              >
                {t("retailPdf")}
              </a>
            )}
            {canWrite && !doc?.cancelled && !doc?.posting_id && (
              <>
                {doc && (
                  <Field
                    label={t("retailReason")}
                    value={reason}
                    disabled={busy}
                    onChange={(e) => setReason(e.target.value)}
                  />
                )}
                <Button
                  disabled={
                    busy || (editing && (!valid || (!!doc && !reason.trim())))
                  }
                  onClick={() => (editing ? save() : setEditing(true))}
                >
                  {t(busy ? "cashSaving" : editing ? "retailSave" : "cashEdit")}
                </Button>
                {doc && (
                  <Button
                    variant="danger"
                    disabled={busy || !reason.trim()}
                    onClick={() => save(true)}
                  >
                    {t("retailCancel")}
                  </Button>
                )}
              </>
            )}
          </div>
        </>
      )}
      {doc && (
        <section className="workspace-panel p-field space-y-control">
          <Heading level={2} size="section">
            {t("retailHistory")}
          </Heading>
          {history.map((v) => (
            <p key={v.revision} className="text-sm">
              {new Date(v.created_at).toLocaleString(language)} ·{" "}
              {t("retailRevision")} {v.revision} ·{" "}
              {v.reason === "post" ? t("retailPosted") : v.reason}
            </p>
          ))}
        </section>
      )}
    </div>
  );
}
