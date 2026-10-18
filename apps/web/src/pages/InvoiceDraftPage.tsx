import { DocumentCurrencyFields } from "../components/organisms/DocumentCurrencyFields";
import { AdvanceDeductions } from "../components/organisms/AdvanceDeductions";
import { AdvanceSettlementPicker } from "../components/organisms/AdvanceSettlementPicker";
import { DocumentAttachments } from "../components/organisms/DocumentAttachments";
import { SalesTemplatePicker } from "../components/organisms/SalesTemplatePicker";
import { CatalogLinePicker } from "../components/organisms/CatalogLinePicker";
import {
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Tabs,
  Icon,
  Alert,
  Badge,
  Button,
  DatePicker,
  Dialog,
  Field,
  Heading,
  Select,
  TextareaField,
} from "@open-faktura/ui";
import {
  createDraft,
  issueInvoice,
  getDraft,
  updateDraft,
  createContact,
  ContactFailure,
  InvoiceCalculationFailure,
  type Company,
  type ContactData,
  type DraftData,
  type InvoiceDraft,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import {
  newDraft,
  emptyParty,
  contactParty,
  addDays,
  invoiceError,
} from "../utils/invoice";
import { emptyContact } from "../utils/contact";
import { useInvoiceCalculation } from "../hooks/useInvoiceCalculation";
import { useAresLookup } from "../hooks/useAresLookup";
import { AresRegistration } from "../components/molecules/AresRegistration";
import { InvoicePartyFields } from "../components/organisms/InvoicePartyFields";
import { IssuedInvoice } from "../components/organisms/IssuedInvoice";
import { InvoiceSummary } from "../components/organisms/InvoiceSummary";
import { InvoiceLines } from "../components/organisms/InvoiceLines";
import { InvoiceCustomerPicker } from "../components/organisms/InvoiceCustomerPicker";
export function InvoiceDraftPage() {
  const { companyId = "", invoiceId = "" } = useParams();
  const [params] = useSearchParams();
  const { company, loading } = useWorkspace();
  const { t } = useAppTranslation();
  if (loading || company?.id !== companyId)
    return (
      <p role="status">{t(loading ? "companiesLoading" : "draftNotFound")}</p>
    );
  return (
    <Editor
      key={`${companyId}:${invoiceId}:${params.get("copy") ?? ""}`}
      company={company}
      id={invoiceId}
      copy={params.get("copy") ?? ""}
    />
  );
}
function Editor({
  company,
  id,
  copy,
}: {
  company: Company;
  id: string;
  copy: string;
}) {
  const { t, language } = useAppTranslation();
  const { onExpired, setPageDirty, navigateAfterSave } = useWorkspace();
  const creating = id === "new";
  const canEdit = company.role !== "viewer";
  const initial = useRef(newDraft(company, language));
  const [data, setData] = useState<DraftData>(initial.current);
  const [draft, setDraft] = useState<InvoiceDraft | null>(null);
  const editable = canEdit && !draft?.number;
  const [settling, setSettling] = useState(false);
  const [issuing, setIssuing] = useState(false);
  const issueKey = useRef(crypto.randomUUID());
  const [loading, setLoading] = useState(!creating || !!copy);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const [fields, setFields] = useState<string[]>([]);
  const [saved, setSaved] = useState(false);
  const [reload, setReload] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const requestId = useRef(crypto.randomUUID());
  const contactRequestId = useRef(crypto.randomUUID());
  const [contactBusy, setContactBusy] = useState(false);
  const [customerExpanded, setCustomerExpanded] = useState(false);
  const dirty =
    JSON.stringify(data) !== JSON.stringify(draft?.data ?? initial.current) ||
    !!copy;
  useEffect(() => {
    setPageDirty(!loading && dirty);
    return () => setPageDirty(false);
  }, [dirty, loading, setPageDirty]);
  const report = (e: unknown) => {
    if (
      (e instanceof InvoiceCalculationFailure || e instanceof ContactFailure) &&
      e.status === 401
    ) {
      onExpired();
      return;
    }
    setError(invoiceError(e));
    setFields(e instanceof InvoiceCalculationFailure ? e.fields : []);
  };
  useEffect(() => {
    if (creating && !copy) return;
    const controller = new AbortController();
    setLoading(true);
    getDraft(apiUrl, company.id, creating ? copy : id, controller.signal)
      .then((v) => {
        if (!controller.signal.aborted) {
          setData(
            creating
              ? {
                  ...v.data,
                  payment: { ...v.data.payment, variable_symbol: "" },
                }
              : v.data,
          );
          if (!creating) setDraft(v);
          setError("");
          setSaved(false);
        }
      })
      .catch((e) => {
        if (!controller.signal.aborted) report(e);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [company.id, id, copy, creating, attempt]);
  const calc = useInvoiceCalculation(
    company.id,
    data.calculation,
    onExpired,
    !draft?.number,
    draft?.advances ? draft.id : undefined,
    draft?.revision,
  );
  const customer = data.customer ?? emptyParty();
  const showCustomerFields =
    customerExpanded ||
    fields.some(
      (field) => field === "customer" || field.startsWith("customer."),
    );
  const customerData: ContactData = {
    ...emptyContact(),
    name: customer.name,
    registration_number: customer.registration_number,
    vat_number: customer.vat_number,
    email: customer.email,
    phone: customer.phone,
    billing_address: customer.address,
  };
  const updateCustomer: Dispatch<SetStateAction<ContactData>> = (action) =>
    setData((current) => {
      const p = current.customer ?? emptyParty();
      const original = { ...emptyContact(), ...p, billing_address: p.address };
      const next = typeof action === "function" ? action(original) : action;
      return {
        ...current,
        customer: {
          name: next.name,
          registration_number: next.registration_number,
          vat_number: next.vat_number,
          email: next.email,
          phone: next.phone,
          address: next.billing_address,
        },
      };
    });
  const ares = useAresLookup(company.id, updateCustomer, onExpired);
  const disabled = !editable || busy || contactBusy;
  const change = (next: DraftData) => {
    setData(next);
    setSaved(false);
  };
  async function save() {
    if (disabled || ares.loading) return;
    setBusy(true);
    setError("");
    setFields([]);
    setSaved(false);
    try {
      const v = creating
        ? await createDraft(apiUrl, company.id, {
            request_id: requestId.current,
            data,
          })
        : await updateDraft(apiUrl, company.id, id, {
            revision: draft!.revision,
            data,
          });
      setDraft(v);
      setData(v.data);
      setSaved(true);
      if (creating)
        navigateAfterSave(`/companies/${company.id}/invoices/${v.id}`);
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  }
  async function issue() {
    if (!draft || disabled || dirty) return;
    setBusy(true);
    setError("");
    setFields([]);
    try {
      const v = await issueInvoice(apiUrl, company.id, id, {
        request_id: issueKey.current,
        revision: draft.revision,
      });
      setDraft(v);
      setData(v.data);
      setIssuing(false);
    } catch (e) {
      report(e);
      setIssuing(false);
    } finally {
      setBusy(false);
    }
  }
  async function saveCustomer() {
    if (disabled || !customer.name.trim()) return;
    setContactBusy(true);
    try {
      const c = await createContact(apiUrl, company.id, {
        request_id: contactRequestId.current,
        data: customerData,
      });
      setData((current) => ({ ...current, contact_id: c.id }));
    } catch (e) {
      report(e);
    } finally {
      setContactBusy(false);
    }
  }
  if (loading) return <p role="status">{t("draftLoading")}</p>;
  if ((!creating && !draft) || (creating && copy && error))
    return (
      <Alert tone="danger" role="alert">
        {t(error || "draftNotFound")}
        <Button variant="secondary" onClick={() => setAttempt((v) => v + 1)}>
          {t("retry")}
        </Button>
      </Alert>
    );
  if (draft?.number) return <IssuedInvoice invoice={draft} canCopy={canEdit} />;
  return (
    <div className="space-y-group">
      <Link
        className="text-sm text-muted hover:underline"
        to={`/companies/${company.id}/invoices`}
      >
        {t("navInvoices")}
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-field">
        <div>
          <Heading level={1} size="page">
            {creating
              ? t(copy ? "draftCopyTitle" : "draftNew")
              : t("draftEditTitle")}
          </Heading>
          <div className="mt-control">
            <Badge>{t("draftListStatus")}</Badge>
          </div>
        </div>
        {!creating && editable && !dirty && (
          <Link
            className="ui-button ui-button-secondary ui-button-md"
            to={`../new?copy=${encodeURIComponent(id)}`}
            relative="path"
          >
            <Icon name="copy" />
            {t("draftCopy")}
          </Link>
        )}
      </div>
      {error && (
        <Alert tone="danger" role="alert">
          <p>{t(error)}</p>
          {fields.length > 0 && (
            <>
              <p>{t("draftCheckFields")}</p>
              <ul className="list-disc pl-group">
                {fields.map((field) => (
                  <li key={field}>{issueFieldLabel(field, t)}</li>
                ))}
              </ul>
            </>
          )}
          {error === "draftConflict" && (
            <Button variant="secondary" onClick={() => setReload(true)}>
              {t("reloadCompany")}
            </Button>
          )}
        </Alert>
      )}
      {saved && (
        <Alert tone="success" role="status">
          {t("draftSaved")}
        </Alert>
      )}
      {creating && editable && (
        <SalesTemplatePicker
          company={company.id}
          disabled={disabled}
          onExpired={onExpired}
          onSelect={(v) => {
            const today = initial.current.issued_on!;
            change({
              ...v.data.invoice,
              issued_on: today,
              due_on: addDays(today, v.data.maturity_days),
              tax_on:
                v.data.invoice.calculation.tax_mode === "vat_payer"
                  ? today
                  : null,
              payment: { ...v.data.invoice.payment, variable_symbol: "" },
            });
          }}
        />
      )}
      <Tabs
        label={t("navInvoices")}
        items={[
          {
            id: "document",
            label: t("invoiceDocumentTab"),
            content: (
              <form
                className="invoice-sheet"
                noValidate
                onSubmit={(e) => {
                  e.preventDefault();
                  void save();
                }}
              >
                <section className="invoice-section">
                  <div className="invoice-party-summary max-w-3xl mx-auto space-y-field">
                    <Heading level={2} size="card">
                      {t("draftCustomer")}
                    </Heading>
                    <div className="space-y-field">
                      <div className="invoice-customer-picker">
                        {editable && (
                          <InvoiceCustomerPicker
                            company={company.id}
                            disabled={disabled}
                            onExpired={onExpired}
                            onSelect={(c) => {
                              setCustomerExpanded(false);
                              ares.cancel();
                              change({
                                ...data,
                                contact_id: c.id,
                                customer: contactParty(c),
                                due_on: data.issued_on
                                  ? addDays(
                                      data.issued_on,
                                      c.data.maturity_days ??
                                        company.settings.maturity_days,
                                    )
                                  : data.due_on,
                                payment: {
                                  ...data.payment,
                                  method:
                                    c.data.payment_method ||
                                    company.settings.payment_method,
                                },
                              });
                            }}
                          />
                        )}
                      </div>
                      {!showCustomerFields && (
                        <div className="flex flex-wrap items-start justify-between gap-field">
                          {customer.name && (
                            <div className="text-sm space-y-inline">
                              <p className="font-semibold">{customer.name}</p>
                              <p className="text-muted">
                                {[
                                  customer.address.street,
                                  customer.address.city,
                                  customer.address.postal_code,
                                ]
                                  .filter(Boolean)
                                  .join(", ")}
                              </p>
                              {customer.registration_number && (
                                <p className="text-muted">
                                  {t("registrationNumber")}:{" "}
                                  {customer.registration_number}
                                </p>
                              )}
                            </div>
                          )}
                          <Button
                            variant="secondary"
                            onClick={() => setCustomerExpanded(true)}
                            aria-expanded={false}
                          >
                            {t(
                              customer.name
                                ? "draftEditCustomer"
                                : "draftManualCustomer",
                            )}
                          </Button>
                        </div>
                      )}
                      {showCustomerFields && (
                        <div className="invoice-parties">
                          <div className="space-y-field">
                            <AresRegistration
                              compact
                              value={customer.registration_number}
                              disabled={disabled}
                              onChange={(registration_number) => {
                                ares.cancel();
                                change({
                                  ...data,
                                  customer: {
                                    ...customer,
                                    registration_number,
                                  },
                                });
                              }}
                              ares={{
                                ...ares,
                                lookup: () => {
                                  if (!disabled) void ares.lookup(customerData);
                                },
                              }}
                            />
                          </div>
                          <div className="space-y-field">
                            <InvoicePartyFields
                              showRegistration={false}
                              value={customer}
                              disabled={disabled}
                              onChange={(next) => {
                                if (
                                  next.registration_number !==
                                  customer.registration_number
                                )
                                  ares.cancel();
                                change({ ...data, customer: next });
                              }}
                            />
                            {editable && !data.contact_id && (
                              <Button
                                type="button"
                                variant="secondary"
                                loading={contactBusy}
                                disabled={disabled || !customer.name.trim()}
                                onClick={() => void saveCustomer()}
                              >
                                {t("draftSaveCustomer")}
                              </Button>
                            )}
                            {data.contact_id && (
                              <p className="text-xs text-muted">
                                {t("draftCustomerSnapshot")}
                              </p>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                    <div className="space-y-inline">
                      <Field
                        horizontal
                        label={t("draftOrder")}
                        value={data.order_number}
                        readOnly={disabled}
                        onChange={(e) =>
                          change({ ...data, order_number: e.target.value })
                        }
                      />
                      <TextareaField
                        horizontal
                        className="invoice-description"
                        label={t("invoiceDescription")}
                        rows={2}
                        value={data.note}
                        readOnly={disabled}
                        onChange={(e) =>
                          change({ ...data, note: e.target.value })
                        }
                      />
                    </div>
                  </div>
                </section>
                <section className="invoice-section invoice-form-facts bg-sidebar grid gap-section md:grid-cols-2">
                  <div className="space-y-field">
                    <Heading level={2} size="card">
                      {t("invoiceDates")}
                    </Heading>
                    <div className="space-y-inline">
                      {(
                        [
                          ["issued_on", "draftIssued"],
                          ["due_on", "draftDue"],
                          ...(data.calculation.tax_mode === "vat_payer"
                            ? [["tax_on", "draftTaxDate"]]
                            : []),
                        ] as ["issued_on" | "due_on" | "tax_on", AppKey][]
                      ).map(([key, label]) => (
                        <DatePicker
                          key={key}
                          label={t(label)}
                          value={data[key] ?? ""}
                          disabled={disabled}
                          onChange={(v) =>
                            change({ ...data, [key]: v || null })
                          }
                        />
                      ))}
                    </div>
                  </div>
                  <div className="space-y-field">
                    <Heading level={2} size="card">
                      {t("invoicePaymentDetails")}
                    </Heading>
                    <div className="space-y-inline">
                      <Select
                        label={t("paymentMethod")}
                        value={data.payment.method}
                        disabled={disabled}
                        options={[
                          { value: "bank_transfer", label: t("bankTransfer") },
                          ...(data.calculation.currency === "CZK"
                            ? [{ value: "cash", label: t("cash") }]
                            : []),
                        ]}
                        onChange={(method) =>
                          change({
                            ...data,
                            payment: {
                              ...data.payment,
                              method: method as "cash" | "bank_transfer",
                            },
                          })
                        }
                      />
                      {data.payment.method === "bank_transfer" && (
                        <>
                          <Field
                            horizontal
                            label={t("contactBankAccount")}
                            value={data.payment.bank_account}
                            readOnly={disabled}
                            onChange={(e) =>
                              change({
                                ...data,
                                payment: {
                                  ...data.payment,
                                  bank_account: e.target.value,
                                },
                              })
                            }
                          />
                          <Field
                            horizontal
                            label={t("draftVariableSymbol")}
                            value={data.payment.variable_symbol}
                            maxLength={30}
                            readOnly={disabled}
                            onChange={(e) =>
                              change({
                                ...data,
                                payment: {
                                  ...data.payment,
                                  variable_symbol: e.target.value,
                                },
                              })
                            }
                          />
                          <Field
                            horizontal
                            label={t("iban")}
                            value={data.payment.iban}
                            readOnly={disabled}
                            onChange={(e) =>
                              change({
                                ...data,
                                payment: {
                                  ...data.payment,
                                  iban: e.target.value,
                                },
                              })
                            }
                          />
                        </>
                      )}
                    </div>
                  </div>
                </section>
                <details
                  className="invoice-section"
                  open={
                    fields.some((f) => f.startsWith("supplier.")) || undefined
                  }
                >
                  <summary className="cursor-pointer text-sm font-medium">
                    {t("draftSettings")}
                  </summary>
                  <Heading level={2} size="card">
                    {t("draftSupplier")}
                  </Heading>
                  <p className="text-sm">
                    {data.supplier.name || t("notProvided")} ·{" "}
                    {data.supplier.address.street} ·{" "}
                    {data.supplier.address.city}
                  </p>
                  <details
                    open={
                      fields.some((f) => f.startsWith("supplier.")) || undefined
                    }
                  >
                    <summary className="cursor-pointer text-sm text-muted">
                      {t("draftEditSupplier")}
                    </summary>
                    <div className="mt-field">
                      <InvoicePartyFields
                        value={data.supplier}
                        disabled={disabled}
                        onChange={(supplier) => change({ ...data, supplier })}
                      />
                    </div>
                  </details>
                  <div className="max-w-sm">
                    <Select
                      label={t("draftLanguage")}
                      value={data.language}
                      disabled={disabled}
                      options={[
                        { value: "cs", label: "Čeština" },
                        { value: "en", label: "English" },
                      ]}
                      onChange={(v) =>
                        change({ ...data, language: v as "cs" | "en" })
                      }
                    />
                  </div>
                </details>
                <DocumentCurrencyFields
                  value={data.calculation}
                  documentDate={data.issued_on}
                  disabled={disabled || !!draft?.advances}
                  onChange={(v) =>
                    change({
                      ...data,
                      payment:
                        v.currency !== "CZK"
                          ? { ...data.payment, method: "bank_transfer" }
                          : data.payment,
                      calculation: {
                        ...data.calculation,
                        ...v,
                        rounding: "none",
                      },
                    })
                  }
                />
                <InvoiceLines
                  catalogPicker={
                    <CatalogLinePicker
                      company={company.id}
                      disabled={disabled}
                      priceMode={data.calculation.price_mode}
                      taxMode={data.calculation.tax_mode}
                      onExpired={onExpired}
                      onSelect={(line) => {
                        setSaved(false);
                        setData((current) => {
                          const lines = current.calculation.lines;
                          const blank =
                            lines.length === 1 &&
                            !lines[0].description.trim() &&
                            !lines[0].unit_price &&
                            lines[0].quantity === "1" &&
                            lines[0].discount_percent === "0";
                          return {
                            ...current,
                            calculation: {
                              ...current.calculation,
                              lines: blank ? [line] : [...lines, line],
                            },
                          };
                        });
                      }}
                    />
                  }
                  value={data.calculation}
                  totals={calc.totals}
                  disabled={disabled}
                  errors={[...fields, ...calc.fields]}
                  onChange={(calculation) => change({ ...data, calculation })}
                />
                {draft?.advances && (
                  <AdvanceDeductions
                    value={draft.advances}
                    company={company.id}
                  />
                )}
                {editable && draft && (
                  <div className="invoice-section">
                    <Button
                      variant="secondary"
                      disabled={busy || dirty}
                      title={dirty ? t("issueSaveFirst") : undefined}
                      onClick={() => setSettling(true)}
                    >
                      {t(draft.advances ? "settlementManage" : "settlementAdd")}
                    </Button>
                  </div>
                )}
                {settling && draft && (
                  <AdvanceSettlementPicker
                    company={company.id}
                    invoice={draft}
                    onClose={() => setSettling(false)}
                    onSaved={() => setAttempt((v) => v + 1)}
                  />
                )}
                <InvoiceSummary
                  calculation={data.calculation}
                  totals={calc.totals}
                  loading={calc.loading}
                  failed={calc.failed}
                />
                {editable ? (
                  <div className="profile-actions rounded-b-control">
                    <Badge>{t("draftStatus")}</Badge>
                    {(creating || dirty) && (
                      <p className="text-sm text-muted">
                        {t("issueSaveFirst")}
                      </p>
                    )}
                    {!creating && (
                      <Button
                        type="button"
                        disabled={disabled || dirty || ares.loading}
                        onClick={() => setIssuing(true)}
                      >
                        {t("issueAction")}
                      </Button>
                    )}
                    <Button
                      type="submit"
                      variant={creating || dirty ? "primary" : "secondary"}
                      loading={busy}
                      disabled={
                        contactBusy || ares.loading || (!creating && !dirty)
                      }
                    >
                      {t("draftSave")}
                    </Button>
                  </div>
                ) : (
                  <p className="p-group text-sm text-muted">
                    {t("draftReadOnly")}
                  </p>
                )}
              </form>
            ),
          },
          {
            id: "attachments",
            label: t("invoiceAttachmentsTab"),
            content: (
              <DocumentAttachments
                company={company.id}
                kind="invoice"
                id={draft?.id}
                editable={canEdit}
              />
            ),
          },
          {
            id: "payments",
            label: t("paymentsTitle"),
            disabled: true,
            description: t("draftPaymentsPending"),
            content: null,
          },
        ]}
      />
      <Dialog
        open={issuing}
        onClose={() => {
          if (!busy) setIssuing(false);
        }}
        title={t("issueAction")}
        description={t("issueConfirm")}
      >
        <p className="text-sm text-muted">
          {t("issueNumberPreview", { number: company.number_preview })}
        </p>
        <div className="mt-group flex justify-end gap-control">
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => setIssuing(false)}
          >
            {t("cancelAction")}
          </Button>
          <Button loading={busy} disabled={busy} onClick={() => void issue()}>
            {t("issueAction")}
          </Button>
        </div>
      </Dialog>
      <Dialog
        open={reload}
        onClose={() => setReload(false)}
        title={t("reloadCompany")}
        description={t("draftDiscard")}
      >
        <div className="flex justify-end gap-control">
          <Button variant="secondary" onClick={() => setReload(false)}>
            {t("cancelAction")}
          </Button>
          <Button
            onClick={() => {
              ares.cancel();
              setReload(false);
              setAttempt((v) => v + 1);
            }}
          >
            {t("reloadCompany")}
          </Button>
        </div>
      </Dialog>
    </div>
  );
}

function issueFieldLabel(
  field: string,
  t: (key: AppKey, params?: Record<string, string | number>) => string,
): string {
  const labels: Record<string, AppKey> = {
    name: "contactName",
    street: "street",
    city: "city",
    postal_code: "postalCode",
    country: "country",
    registration_number: "registrationNumber",
    vat_number: "vatNumber",
    issued_on: "draftIssued",
    due_on: "draftDue",
    tax_on: "draftTaxDate",
    "payment.bank_account": "contactBankAccount",
    "payment.iban": "iban",
    "payment.variable_symbol": "draftVariableSymbol",
    number_year: "issueYearError",
    next_number: "issueSequenceExhausted",
    tax_mode: "issueTaxModeError",
    customer: "draftCustomer",
    lines: "draftItems",
  };
  if (field.startsWith("supplier.") || field.startsWith("customer.")) {
    const [party, key] = field.split(".");
    return `${t(party === "supplier" ? "draftSupplier" : "draftCustomer")}: ${t(labels[key] ?? "draftCheckFields")}`;
  }
  if (field.startsWith("lines.")) {
    const [, index, key] = field.split(".");
    const names: Record<string, AppKey> = {
      description: "draftDescription",
      quantity: "draftQuantity",
      unit_price: "draftUnitPrice",
      vat_percent: "draftVat",
      discount_percent: "draftDiscount",
    };
    return `${t("draftItem", { number: Number(index) + 1 })}: ${t(names[key] ?? "draftCheckFields")}`;
  }
  return t(labels[field] ?? "draftCheckFields");
}
