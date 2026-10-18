import { DocumentExports } from "../components/organisms/DocumentExports";
import { DocumentTags } from "../components/organisms/DocumentTags";
import { DocumentCurrencyFields } from "../components/organisms/DocumentCurrencyFields";
import { AdvanceDetail } from "../components/organisms/AdvanceDetail";
import { DocumentAttachments } from "../components/organisms/DocumentAttachments";
import { QuoteDelivery } from "../components/organisms/QuoteDelivery";
import { SalesOverview } from "../components/organisms/SalesOverview";
import { SalesTemplatePicker } from "../components/organisms/SalesTemplatePicker";
import { useEffect, useRef, useState } from "react";
import { Link, useParams, useSearchParams, useLocation } from "react-router";
import {
  Alert,
  Badge,
  Button,
  DatePicker,
  Field,
  Heading,
  Select,
  Tabs,
} from "@open-faktura/ui";
import {
  createSales,
  getSales,
  updateSales,
  convertSales,
  SalesFailure,
  type SalesDocument,
  type SalesData,
  type SalesKind,
  type Company,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import { newDraft, emptyParty, contactParty, addDays } from "../utils/invoice";
import { salesError, salesStates } from "../utils/sales";
import { InvoiceLines } from "../components/organisms/InvoiceLines";
import { InvoiceSummary } from "../components/organisms/InvoiceSummary";
import { InvoiceCustomerPicker } from "../components/organisms/InvoiceCustomerPicker";
import { InvoicePartyFields } from "../components/organisms/InvoicePartyFields";
import { CatalogLinePicker } from "../components/organisms/CatalogLinePicker";
import { useInvoiceCalculation } from "../hooks/useInvoiceCalculation";
export function SalesDetailPage({ kind }: { kind: SalesKind }) {
  const { companyId = "", salesId = "" } = useParams();
  const { company } = useWorkspace();
  const { t } = useAppTranslation();
  const [params] = useSearchParams();
  const location = useLocation();
  if (company?.id !== companyId)
    return <p role="status">{t("salesLoading")}</p>;
  return (
    <Editor
      key={`${companyId}:${kind}:${salesId}:${location.key}`}
      company={company}
      kind={kind}
      id={salesId}
      template={params.get("template") ?? ""}
    />
  );
}
function Editor({
  company,
  kind,
  id,
  template,
}: {
  company: Company;
  kind: SalesKind;
  id: string;
  template: string;
}) {
  const { t, language } = useAppTranslation();
  const { onExpired, setPageDirty, navigateAfterSave } = useWorkspace();
  const creating = id === "new";
  const initial = useRef<SalesData>({
    name: "",
    maturity_days: company.settings.maturity_days,
    status: "created",
    invoice: {
      ...newDraft(company, language),
      ...(kind === "advance" ? { tax_on: null } : {}),
    },
  });
  const [data, setData] = useState(initial.current);
  const [record, setRecord] = useState<SalesDocument | null>(null);
  const [loading, setLoading] = useState(!creating);
  const [editing, setEditing] = useState(creating);
  const editingRef = useRef(editing);
  editingRef.current = editing;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const [attempt, setAttempt] = useState(0);
  const [saved, setSaved] = useState(false);
  const requestId = useRef(crypto.randomUUID());
  const conversionId = useRef(crypto.randomUUID());
  const canWrite = company.role !== "viewer";
  const dirty =
    JSON.stringify(data) !== JSON.stringify(record?.data ?? initial.current);
  const disabled = !editing || busy || !canWrite || !!record?.invoice_id;
  const base = `/companies/${company.id}/${kind === "advance" ? "advances" : kind === "quote" ? "quotes" : "templates"}`;
  useEffect(() => {
    setPageDirty(dirty);
    return () => setPageDirty(false);
  }, [dirty, setPageDirty]);
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    setError(salesError(e));
  }
  useEffect(() => {
    if (creating) return;
    const c = new AbortController();
    setLoading(true);
    getSales(apiUrl, company.id, id, c.signal)
      .then((v) => {
        if (!c.signal.aborted) {
          if (v.kind !== kind) {
            setError("salesNotFound");
            return;
          }
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
    return () => c.abort();
  }, [id, company.id, kind, creating, attempt]);
  function applyTemplate(v: SalesDocument) {
    const today = initial.current.invoice.issued_on!;
    setData({
      ...v.data,
      status: "created",
      invoice: {
        ...v.data.invoice,
        issued_on: today,
        due_on: addDays(today, v.data.maturity_days),
        tax_on:
          v.data.invoice.calculation.tax_mode === "vat_payer" ? today : null,
        payment: { ...v.data.invoice.payment, variable_symbol: "" },
      },
    });
  }
  useEffect(() => {
    if (!creating || !template) return;
    const c = new AbortController();
    setLoading(true);
    getSales(apiUrl, company.id, template, c.signal)
      .then((v) => {
        if (!c.signal.aborted) {
          if (v.kind === "template" && !v.archived) applyTemplate(v);
          else setError("salesNotFound");
        }
      })
      .catch((e) => {
        if (!c.signal.aborted) failure(e);
      })
      .finally(() => {
        if (!c.signal.aborted) setLoading(false);
      });
    return () => c.abort();
  }, [template, company.id, creating]);
  const change = (patch: Partial<SalesData>) => {
    setData((v) => ({ ...v, ...patch }));
    setSaved(false);
  };
  const invoice = (patch: Partial<SalesData["invoice"]>) =>
    change({ invoice: { ...data.invoice, ...patch } });
  const calculation = useInvoiceCalculation(
    company.id,
    data.invoice.calculation,
    onExpired,
    !loading && editing,
  );
  const totals = editing ? calculation.totals : (record?.totals ?? null);
  async function save() {
    setBusy(true);
    setError("");
    try {
      const v = record
        ? await updateSales(apiUrl, company.id, id, {
            revision: record.revision,
            archived: record.archived,
            data,
          })
        : await createSales(apiUrl, company.id, {
            request_id: requestId.current,
            kind,
            data:
              kind === "advance"
                ? { ...data, invoice: { ...data.invoice, tax_on: null } }
                : data,
          });
      setRecord(v);
      setData(v.data);
      setEditing(false);
      setSaved(true);
      if (creating) navigateAfterSave(`${base}/${v.id}`);
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  async function archive() {
    if (!record) return;
    setBusy(true);
    try {
      const v = await updateSales(apiUrl, company.id, id, {
        revision: record.revision,
        archived: !record.archived,
        data: record.data,
      });
      setRecord(v);
      setData(v.data);
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  async function convert() {
    if (!record) return;
    setBusy(true);
    setError("");
    try {
      const v = await convertSales(apiUrl, company.id, id, {
        request_id: conversionId.current,
        revision: record.revision,
        issued_on: initial.current.invoice.issued_on!,
      });
      navigateAfterSave(`/companies/${company.id}/invoices/${v.id}`);
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  if (loading) return <p role="status">{t("salesLoading")}</p>;
  if (!creating && !record)
    return (
      <Alert tone="danger">
        {t(error || "salesNotFound")}{" "}
        <Button onClick={() => setAttempt((v) => v + 1)}>{t("retry")}</Button>
      </Alert>
    );
  return (
    <div className="space-y-field">
      <Link className="text-sm text-muted hover:underline" to={base}>
        {t("salesBack")}
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-field">
        <Heading level={1} size="page">
          {record?.number ??
            (creating
              ? t(
                  kind === "advance"
                    ? "advanceNew"
                    : kind === "quote"
                      ? "salesNewQuote"
                      : "salesNewTemplate",
                )
              : data.name)}
        </Heading>
        <div className="flex flex-wrap gap-inline">
          {record && kind === "quote" && (
            <Badge>{t(salesStates[data.status] ?? "salesCreated")}</Badge>
          )}
          {record && !editing && canWrite && (
            <>
              {!record.invoice_id &&
                !(kind === "advance" && Number(record.advance_paid) > 0) && (
                  <Button variant="secondary" onClick={() => setEditing(true)}>
                    {t("salesEdit")}
                  </Button>
                )}
              <Button
                variant="ghost"
                disabled={
                  busy ||
                  (kind === "advance" && Number(record.advance_paid) > 0)
                }
                onClick={() => void archive()}
              >
                {t(record.archived ? "salesRestore" : "salesArchive")}
              </Button>
              {record.invoice_id ? (
                <Link
                  className="ui-button ui-button-primary ui-button-md"
                  to={`/companies/${company.id}/invoices/${record.invoice_id}`}
                >
                  {t("salesLinked")}
                </Link>
              ) : (
                kind !== "advance" &&
                !record.archived && (
                  <Button disabled={busy} onClick={() => void convert()}>
                    {t(kind === "quote" ? "salesConvert" : "salesUse")}
                  </Button>
                )
              )}
              {kind === "template" && !record.archived && (
                <Link
                  className="ui-button ui-button-secondary ui-button-md"
                  to={`/companies/${company.id}/quotes/new?template=${id}`}
                >
                  {t("salesUseForQuote")}
                </Link>
              )}
            </>
          )}
        </div>
      </div>
      {kind === "advance" && <Alert>{t("advanceNotice")}</Alert>}
      {error && (
        <Alert tone="danger" role="alert">
          {t(error)}
          {error === "salesConflict" && (
            <Button onClick={() => setAttempt((v) => v + 1)}>
              {t("reloadCompany")}
            </Button>
          )}
        </Alert>
      )}
      {saved && (
        <Alert tone="success" role="status">
          {t("salesSaved")}
        </Alert>
      )}
      {kind !== "advance" &&
        record &&
        !editing &&
        !record.invoice_id &&
        !record.archived && (
          <p className="text-sm text-muted">{t("salesConvertHint")}</p>
        )}
      {!editing && record ? (
        <>
          {kind === "advance" ? (
            <AdvanceDetail
              document={record}
              onChanged={() => setAttempt((v) => v + 1)}
            />
          ) : kind === "quote" ? (
            <QuoteDelivery
              document={record}
              canSend={canWrite && !record.archived}
              onChanged={() => {
                void getSales(apiUrl, company.id, id)
                  .then((v) => {
                    if (!editingRef.current) {
                      setRecord(v);
                      setData(v.data);
                    }
                  })
                  .catch(failure);
              }}
            >
              <Tabs
                label={t("invoiceDocumentTab")}
                items={[
                  {
                    id: "document",
                    label: t("invoiceDocumentTab"),
                    content: <SalesOverview document={record} />,
                  },
                  {
                    id: "attachments",
                    label: t("invoiceAttachmentsTab"),
                    content: (
                      <DocumentAttachments
                        company={company.id}
                        kind="quote"
                        id={record.id}
                        editable={canWrite && !record.archived}
                      />
                    ),
                  },
                ]}
              />
            </QuoteDelivery>
          ) : (
            <SalesOverview document={record} />
          )}
        </>
      ) : (
        <form
          noValidate
          className="invoice-sheet"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <section className="invoice-section">
            <div className="invoice-party-summary max-w-3xl mx-auto space-y-field">
              {kind === "quote" && creating && (
                <SalesTemplatePicker
                  company={company.id}
                  disabled={disabled}
                  onExpired={onExpired}
                  onSelect={applyTemplate}
                />
              )}
              <Field
                horizontal
                label={t(
                  kind === "advance"
                    ? "advanceDescription"
                    : kind === "quote"
                      ? "salesDescription"
                      : "salesName",
                )}
                required
                maxLength={200}
                readOnly={disabled}
                value={data.name}
                onChange={(e) => change({ name: e.target.value })}
              />
              {kind === "template" && (
                <Field
                  horizontal
                  label={t("invoiceDescription")}
                  readOnly={disabled}
                  value={data.invoice.note}
                  onChange={(e) => invoice({ note: e.target.value })}
                />
              )}
              <Heading level={2} size="card">
                {t("draftCustomer")}
              </Heading>
              {!disabled && (
                <InvoiceCustomerPicker
                  company={company.id}
                  disabled={disabled}
                  onExpired={onExpired}
                  onSelect={(c) =>
                    invoice({ contact_id: c.id, customer: contactParty(c) })
                  }
                />
              )}
              <p className="font-medium">
                {data.invoice.customer?.name || "—"}
              </p>
              <details>
                <summary className="text-sm cursor-pointer">
                  {t("salesCustomerDetails")}
                </summary>
                <div className="mt-field">
                  <InvoicePartyFields
                    value={data.invoice.customer ?? emptyParty()}
                    disabled={disabled}
                    onChange={(v) => invoice({ customer: v, contact_id: null })}
                  />
                </div>
              </details>
              {kind === "quote" && (
                <Select
                  label={t("salesStatus")}
                  disabled={disabled}
                  value={data.status}
                  options={Object.entries(salesStates).map(
                    ([value, label]) => ({
                      value,
                      label: t(label),
                    }),
                  )}
                  onChange={(status) =>
                    change({ status: status as SalesData["status"] })
                  }
                />
              )}
              {kind === "quote" && !disabled && (
                <p className="text-xs text-muted">{t("salesSentHint")}</p>
              )}
              <Field
                horizontal
                label={t("draftOrder")}
                readOnly={disabled}
                value={data.invoice.order_number}
                onChange={(e) => invoice({ order_number: e.target.value })}
              />
            </div>
          </section>
          <section className="invoice-section invoice-form-facts bg-sidebar grid gap-section md:grid-cols-2">
            <div className="space-y-field">
              <Heading level={2} size="card">
                {t("salesDates")}
              </Heading>
              {kind !== "template" && (
                <>
                  <DatePicker
                    label={t("draftIssued")}
                    disabled={disabled}
                    value={data.invoice.issued_on ?? ""}
                    onChange={(v) => invoice({ issued_on: v || null })}
                  />
                  <DatePicker
                    label={t(
                      kind === "advance" ? "draftDue" : "salesValidUntil",
                    )}
                    disabled={disabled}
                    value={data.invoice.due_on ?? ""}
                    onChange={(v) => invoice({ due_on: v || null })}
                  />
                </>
              )}
              <Field
                horizontal
                label={t("salesMaturity")}
                type="number"
                min={0}
                max={365}
                readOnly={disabled}
                value={data.maturity_days}
                onChange={(e) =>
                  change({ maturity_days: Number(e.target.value) })
                }
              />
            </div>
            <div className="space-y-field">
              <Heading level={2} size="card">
                {t("salesPayment")}
              </Heading>
              <Select
                label={t("paymentMethod")}
                disabled={disabled}
                value={data.invoice.payment.method}
                options={[
                  { value: "bank_transfer", label: t("bankTransfer") },
                  ...(data.invoice.calculation.currency === "CZK"
                    ? [{ value: "cash", label: t("cash") }]
                    : []),
                ]}
                onChange={(method) =>
                  invoice({
                    payment: {
                      ...data.invoice.payment,
                      method: method as "cash" | "bank_transfer",
                    },
                  })
                }
              />
              <Field
                horizontal
                label={t("contactBankAccount")}
                readOnly={disabled}
                value={data.invoice.payment.bank_account}
                onChange={(e) =>
                  invoice({
                    payment: {
                      ...data.invoice.payment,
                      bank_account: e.target.value,
                    },
                  })
                }
              />
              <Field
                horizontal
                label={t("iban")}
                readOnly={disabled}
                value={data.invoice.payment.iban}
                onChange={(e) =>
                  invoice({
                    payment: { ...data.invoice.payment, iban: e.target.value },
                  })
                }
              />
              <Select
                label={t("draftLanguage")}
                disabled={disabled}
                value={data.invoice.language}
                options={[
                  { value: "cs", label: "Čeština" },
                  { value: "en", label: "English" },
                ]}
                onChange={(v) => invoice({ language: v as "cs" | "en" })}
              />
            </div>
          </section>
          <section className="invoice-section">
            <details>
              <summary className="text-sm cursor-pointer">
                {t("draftSupplier")}
              </summary>
              <div className="mt-field">
                <InvoicePartyFields
                  value={data.invoice.supplier}
                  disabled={disabled}
                  onChange={(supplier) => invoice({ supplier })}
                />
              </div>
            </details>
          </section>
          <section className="invoice-section">
            <DocumentCurrencyFields
              value={data.invoice.calculation}
              documentDate={data.invoice.issued_on}
              disabled={disabled}
              onChange={(value) =>
                invoice({
                  calculation: {
                    ...data.invoice.calculation,
                    ...value,
                    rounding: "none",
                  },
                  payment: {
                    ...data.invoice.payment,
                    method:
                      value.currency === "CZK"
                        ? data.invoice.payment.method
                        : "bank_transfer",
                  },
                })
              }
            />
          </section>
          <InvoiceLines
            title={t("salesItems")}
            value={data.invoice.calculation}
            disabled={disabled}
            errors={calculation.fields}
            totals={totals}
            onChange={(v) => invoice({ calculation: v })}
            catalogPicker={
              <CatalogLinePicker
                company={company.id}
                priceMode={data.invoice.calculation.price_mode}
                taxMode={data.invoice.calculation.tax_mode}
                disabled={disabled}
                onExpired={onExpired}
                onSelect={(line) => {
                  setData((v) => ({
                    ...v,
                    invoice: {
                      ...v.invoice,
                      calculation: {
                        ...v.invoice.calculation,
                        lines:
                          v.invoice.calculation.lines.length === 1 &&
                          !v.invoice.calculation.lines[0].description &&
                          !v.invoice.calculation.lines[0].unit_price
                            ? [line]
                            : [...v.invoice.calculation.lines, line],
                      },
                    },
                  }));
                  setSaved(false);
                }}
              />
            }
          />
          <InvoiceSummary
            totalLabel={kind === "quote" ? t("quoteTotalPrice") : undefined}
            calculation={data.invoice.calculation}
            totals={totals}
            loading={editing && calculation.loading}
            failed={editing && calculation.failed}
          />
          {editing && canWrite && (
            <div className="invoice-section flex justify-end gap-inline">
              {record && (
                <Button
                  variant="secondary"
                  disabled={busy}
                  onClick={() => {
                    setData(record.data);
                    setEditing(false);
                    setError("");
                  }}
                >
                  {t("salesCancel")}
                </Button>
              )}
              <Button type="submit" disabled={disabled}>
                {t(
                  kind === "advance" && creating ? "advanceIssue" : "salesSave",
                )}
              </Button>
            </div>
          )}
        </form>
      )}
      {editing && kind === "quote" && (
        <div className="workspace-panel">
          <DocumentAttachments
            company={company.id}
            kind="quote"
            id={record?.id}
            editable={canWrite && !record?.archived}
            disabled={busy}
          />
        </div>
      )}
      {record && (
        <DocumentTags
          key={`sales-tags-${record.id}`}
          company={company.id}
          editable={canWrite}
          target={{ kind: "sales", id: record.id }}
        />
      )}
      {record && (
        <DocumentExports
          key={`exports-${record.id}`}
          company={company.id}
          target={{ kind: "sales", id: record.id, revision: record.revision }}
          editable={canWrite}
          owner={company.role === "owner"}
        />
      )}
    </div>
  );
}
