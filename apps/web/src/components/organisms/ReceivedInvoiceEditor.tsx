import { DocumentCurrencyFields } from "./DocumentCurrencyFields";
import { useState, useEffect } from "react";
import { Link } from "react-router";
import {
  Alert,
  Badge,
  Button,
  DatePicker,
  Dialog,
  Field,
  Heading,
  Select,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  stockCards,
  type StockCard,
  type ReceivedCommand,
} from "@open-faktura/api-client";
import { apiUrl } from "../../services/status";
import { useReceivedInvoice } from "../../hooks/useReceivedInvoice";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../../locales";
import { moneyText, contactParty } from "../../utils/invoice";
import { DocumentAttachments } from "./DocumentAttachments";
import { InvoiceCustomerPicker } from "./InvoiceCustomerPicker";
export function ReceivedInvoiceEditor({
  companyId,
  id,
  kind = "invoice",
}: {
  companyId: string;
  id: string;
  kind?: "invoice" | "receipt";
}) {
  const m = useReceivedInvoice(companyId, id, kind),
    {
      document: d,
      data,
      setData,
      loading,
      busy,
      error,
      dirty,
      preview,
      command,
    } = m;
  const { company, onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [editing, setEditing] = useState(id === "new"),
    [prompt, setPrompt] = useState<"pay" | "reverse" | "cancel" | null>(null),
    [paymentId, setPaymentId] = useState(""),
    [amount, setAmount] = useState(""),
    [date, setDate] = useState(""),
    [method, setMethod] = useState("bank_transfer"),
    [reason, setReason] = useState("");
  const [stock, setStock] = useState<StockCard[]>([]);
  const [stockError, setStockError] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    setStock([]);
    setStockError(false);
    stockCards(apiUrl, companyId, controller.signal)
      .then((v) => {
        if (!controller.signal.aborted) setStock(v);
      })
      .catch(() => {
        if (!controller.signal.aborted) setStockError(true);
      });
    return () => controller.abort();
  }, [companyId]);
  const receipt = kind === "receipt";
  const write = company?.id === companyId && company.role !== "viewer",
    editable = write && !d?.cancelled && (!d || receipt || d.paid === "0.00"),
    disabled = !editing || !editable || busy;
  const labels = {
    pay: "paymentAdd",
    reverse: "paymentReverse",
    cancel: "receivedCancel",
  } as const;
  const field = (
    key:
      | "external_number"
      | "variable_symbol"
      | "bank_account"
      | "description"
      | "note",
    label: AppKey,
    required = false,
  ) => (
    <Field
      label={t(label)}
      value={data[key]}
      required={required}
      disabled={disabled}
      onChange={(e) => setData({ ...data, [key]: e.target.value })}
    />
  );
  function open(action: "pay" | "reverse" | "cancel", payment = "") {
    setPrompt(action);
    setPaymentId(payment);
    setReason("");
    setAmount(d?.remaining ?? "");
    setDate(
      new Intl.DateTimeFormat("sv-SE", { timeZone: "Europe/Prague" }).format(
        new Date(),
      ),
    );
  }
  async function submit() {
    if (!prompt) return;
    const action: Omit<ReceivedCommand, "request_id" | "revision"> =
      prompt === "pay"
        ? {
            action: "pay",
            payment: {
              amount,
              paid_on: date,
              method: method === "cash" ? "cash" : "bank_transfer",
              note: reason,
            },
          }
        : prompt === "reverse"
          ? { action: "reverse", payment_id: paymentId, reason }
          : { action: "cancel", reason };
    if (await command(action)) setPrompt(null);
  }
  if (loading) return <p role="status">{t("draftLoading")}</p>;
  if (id !== "new" && !d)
    return (
      <Alert tone="danger" role="alert">
        {t(error || "requestFailed")}{" "}
        <Button onClick={m.reload}>{t("retry")}</Button>
      </Alert>
    );
  return (
    <div className="space-y-group">
      {!receipt && (
        <DocumentCurrencyFields
          value={data}
          documentDate={data.issued_on}
          disabled={!editing || busy || !write}
          onChange={(v) => setData({ ...data, ...v })}
        />
      )}
      {m.inboxId && id === "new" && (
        <Alert>
          {t("inboxReview")}{" "}
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => m.extractInbox()}
          >
            {t("inboxExtract")}
          </Button>
        </Alert>
      )}
      <div className="flex flex-wrap items-center justify-between gap-field">
        <div>
          <Link
            className="text-sm text-muted underline"
            to={`/companies/${companyId}/${receipt ? "receipts" : "received-invoices"}`}
          >
            {t(receipt ? "receiptsTitle" : "receivedTitle")}
          </Link>
          <Heading level={1} size="page">
            {d
              ? `${t(receipt ? "receiptSingle" : "receivedSingle")} ${d.number}`
              : t(receipt ? "receiptsNew" : "receivedNew")}
          </Heading>
        </div>
        {d && (
          <Badge>
            {t(
              d.cancelled
                ? "receivedCancelled"
                : d.payment_status === "paid"
                  ? "paymentPaid"
                  : d.payment_status === "partial"
                    ? "paymentPartial"
                    : "paymentUnpaid",
            )}
          </Badge>
        )}
      </div>
      {error && (
        <Alert tone="danger" role="alert">
          {t(error)}{" "}
          <Button variant="ghost" disabled={busy} onClick={m.reload}>
            {t("retry")}
          </Button>
        </Alert>
      )}
      {d && !editing && (
        <div className="flex flex-wrap gap-control">
          {editable && (
            <Button variant="secondary" onClick={() => setEditing(true)}>
              {t("receivedEdit")}
            </Button>
          )}
          {write && !d.cancelled && (
            <>
              {!receipt && (
                <Button
                  disabled={d.remaining === "0.00"}
                  onClick={() => open("pay")}
                >
                  {t("paymentAdd")}
                </Button>
              )}
              <Button
                variant="ghost"
                disabled={!receipt && d.paid !== "0.00"}
                onClick={() => open("cancel")}
              >
                {t("receivedCancel")}
              </Button>
            </>
          )}
        </div>
      )}
      <form
        className="invoice-sheet"
        onSubmit={async (e) => {
          e.preventDefault();
          if (await command({ action: "update", data })) setEditing(false);
        }}
      >
        <div className="grid gap-group p-field lg:grid-cols-2">
          <section className="space-y-field">
            <Heading level={2} size="section">
              {t("draftSupplier")}
            </Heading>
            {editing && (
              <InvoiceCustomerPicker
                label={t("receivedFindSupplier")}
                company={companyId}
                disabled={disabled}
                onExpired={onExpired}
                onSelect={(c) =>
                  setData({
                    ...data,
                    supplier_contact_id: c.id,
                    supplier: contactParty(c),
                    bank_account: receipt
                      ? ""
                      : c.data.iban || c.data.bank_account,
                  })
                }
              />
            )}
            <Field
              label={t("receivedSupplierName")}
              value={data.supplier.name}
              required={!receipt}
              disabled={disabled}
              onChange={(e) =>
                setData({
                  ...data,
                  supplier: { ...data.supplier, name: e.target.value },
                })
              }
            />
            <div className="grid grid-cols-2 gap-field">
              {(["registration_number", "vat_number"] as const).map((k) => (
                <Field
                  key={k}
                  label={t(
                    k === "registration_number" ? "receivedIco" : "receivedDic",
                  )}
                  value={data.supplier[k]}
                  disabled={disabled}
                  onChange={(e) =>
                    setData({
                      ...data,
                      supplier: { ...data.supplier, [k]: e.target.value },
                    })
                  }
                />
              ))}
            </div>
            <details>
              <summary className="cursor-pointer text-sm font-medium">
                {t("receivedSupplierDetails")}
              </summary>
              <div className="grid gap-field py-field sm:grid-cols-2">
                {(["street", "city", "postal_code", "country"] as const).map(
                  (k) => (
                    <Field
                      key={k}
                      label={t(
                        (
                          {
                            street: "receivedStreet",
                            city: "receivedCity",
                            postal_code: "receivedPostal",
                            country: "receivedCountry",
                          } as const
                        )[k],
                      )}
                      value={data.supplier.address[k]}
                      disabled={disabled}
                      onChange={(e) =>
                        setData({
                          ...data,
                          supplier: {
                            ...data.supplier,
                            address: {
                              ...data.supplier.address,
                              [k]: e.target.value,
                            },
                          },
                        })
                      }
                    />
                  ),
                )}
              </div>
            </details>
          </section>
          <section className="space-y-field">
            <Heading level={2} size="section">
              {t("receivedDocument")}
            </Heading>
            <div className="grid gap-field sm:grid-cols-2">
              {field("external_number", "receivedExternal", !receipt)}
              {!receipt && field("variable_symbol", "receivedVs")}
              {(["issued_on", "received_on", "due_on", "tax_on"] as const)
                .filter((k) => !receipt || k === "issued_on" || k === "tax_on")
                .map((k) => (
                  <DatePicker
                    key={k}
                    label={t(
                      (
                        {
                          issued_on: "draftIssued",
                          received_on: "receivedReceived",
                          due_on: "draftDue",
                          tax_on: "receivedTaxDate",
                        } as const
                      )[k],
                    )}
                    value={data[k] ?? ""}
                    disabled={disabled}
                    required={k !== "tax_on"}
                    onChange={(v) =>
                      setData({
                        ...data,
                        [k]: k === "tax_on" ? v || null : v,
                        ...(receipt && k === "issued_on"
                          ? { received_on: v, due_on: v }
                          : {}),
                      })
                    }
                  />
                ))}
            </div>
            {!receipt && field("bank_account", "receivedBank")}
            {receipt && (
              <Select
                label={t("paymentMethod")}
                value={data.receipt_method ?? "card"}
                disabled={disabled}
                options={[
                  { value: "card", label: t("receiptCard") },
                  ...(data.currency === "CZK"
                    ? [{ value: "cash", label: t("receivedCash") }]
                    : []),
                ]}
                onChange={(v) =>
                  setData({
                    ...data,
                    receipt_method: v === "cash" ? "cash" : "card",
                  })
                }
              />
            )}
            {receipt && (
              <p className="text-sm text-muted">{t("receiptPaidHint")}</p>
            )}
            {field("description", "receivedDescription")}
          </section>
        </div>
        <div className="border-t border-border p-field space-y-field">
          <Heading level={2} size="section">
            {t("receivedAmounts")}
          </Heading>
          <p className="text-sm text-muted">{t("receivedAmountsHint")}</p>
          <div
            className="max-w-full overflow-x-auto"
            tabIndex={0}
            role="region"
            aria-label={t("receivedAmounts")}
          >
            <Table className="min-w-2xl">
              <TableHeader>
                <TableRow>
                  {[
                    "receivedLine",
                    "receivedRate",
                    "receivedNet",
                    "draftVat",
                    "receivedActions",
                  ].map((k) => (
                    <TableHead key={k}>{t(k as AppKey)}</TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <tbody>
                {data.lines.map((line, i) => (
                  <TableRow key={i}>
                    <TableCell>
                      <Field
                        label={`${t("receivedLine")} ${i + 1}`}
                        value={line.description}
                        required
                        disabled={disabled}
                        onChange={(e) =>
                          setData({
                            ...data,
                            lines: data.lines.map((v, j) =>
                              j === i
                                ? { ...v, description: e.target.value }
                                : v,
                            ),
                          })
                        }
                      />
                      <Select
                        label={`${t("stockReceiveItem")} ${i + 1}`}
                        value={line.stock?.catalog_id ?? ""}
                        disabled={disabled || stockError}
                        options={[
                          { value: "", label: t("stockNoReceipt") },
                          ...stock.map((c) => ({
                            value: c.catalog_id,
                            label: c.item.name,
                          })),
                        ]}
                        onChange={(catalog) => {
                          const selected = stock.find(
                            (c) => c.catalog_id === catalog,
                          );
                          setData({
                            ...data,
                            lines: data.lines.map((v, j) =>
                              j === i
                                ? {
                                    ...v,
                                    stock: selected
                                      ? {
                                          catalog_id: catalog,
                                          quantity: line.stock?.quantity ?? "1",
                                          unit: selected.item.unit,
                                        }
                                      : null,
                                  }
                                : v,
                            ),
                          });
                        }}
                      />
                      {stockError && (
                        <p className="text-sm text-muted">{t("stockFailed")}</p>
                      )}
                      {line.stock && (
                        <>
                          <Field
                            label={`${t("stockReceiveQuantity")} ${i + 1} (${line.stock.unit})`}
                            inputMode="decimal"
                            value={line.stock.quantity}
                            disabled={disabled}
                            onChange={(e) =>
                              setData({
                                ...data,
                                lines: data.lines.map((v, j) =>
                                  j === i
                                    ? {
                                        ...v,
                                        stock: {
                                          ...line.stock!,
                                          quantity: e.target.value.replace(
                                            ",",
                                            ".",
                                          ),
                                        },
                                      }
                                    : v,
                                ),
                              })
                            }
                          />
                          <Field
                            label={`${t("stockAcquisitionCost")} ${i + 1}`}
                            inputMode="decimal"
                            value={line.stock.acquisition_unit_cost ?? ""}
                            disabled={disabled}
                            onChange={(e) =>
                              setData({
                                ...data,
                                lines: data.lines.map((v, j) =>
                                  j === i
                                    ? {
                                        ...v,
                                        stock: {
                                          ...line.stock!,
                                          acquisition_unit_cost:
                                            e.target.value.trim()
                                              ? e.target.value.replace(",", ".")
                                              : null,
                                        },
                                      }
                                    : v,
                                ),
                              })
                            }
                          />
                          <p className="text-sm text-muted">
                            {t("stockAcquisitionHelp")}
                          </p>
                        </>
                      )}
                    </TableCell>
                    <TableCell>
                      <Select
                        label={`${t("receivedRate")} ${i + 1}`}
                        value={line.rate}
                        disabled={disabled}
                        options={[
                          "0",
                          "12",
                          "21",
                          ...(!["0", "12", "21"].includes(line.rate)
                            ? [line.rate]
                            : []),
                        ].map((v) => ({ value: v, label: `${v} %` }))}
                        onChange={(rate) =>
                          setData({
                            ...data,
                            lines: data.lines.map((v, j) =>
                              j === i ? { ...v, rate } : v,
                            ),
                          })
                        }
                      />
                    </TableCell>
                    {(["net", "vat"] as const).map((k) => (
                      <TableCell key={k}>
                        <Field
                          label={`${t(k === "net" ? "receivedNet" : "draftVat")} ${i + 1}`}
                          value={line[k]}
                          required
                          inputMode="decimal"
                          disabled={disabled}
                          onChange={(e) =>
                            setData({
                              ...data,
                              lines: data.lines.map((v, j) =>
                                j === i
                                  ? {
                                      ...v,
                                      [k]: e.target.value.replace(",", "."),
                                    }
                                  : v,
                              ),
                            })
                          }
                        />
                      </TableCell>
                    ))}
                    <TableCell>
                      {editing && (
                        <Button
                          variant="ghost"
                          disabled={disabled || data.lines.length === 1}
                          aria-label={`${t("receivedRemoveLine")} ${i + 1}`}
                          onClick={() =>
                            setData({
                              ...data,
                              lines: data.lines.filter((_, j) => j !== i),
                            })
                          }
                        >
                          {t("receivedRemoveLine")}
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </tbody>
            </Table>
          </div>
          {editing && (
            <Button
              variant="secondary"
              disabled={disabled || data.lines.length >= 100}
              onClick={() =>
                setData({
                  ...data,
                  lines: [
                    ...data.lines,
                    { description: "", rate: "21", net: "", vat: "" },
                  ],
                })
              }
            >
              {t("receivedAddLine")}
            </Button>
          )}
          <div className="flex flex-wrap items-end justify-between gap-field">
            {data.lines.some((line) => line.stock) && (
              <div className="space-y-control">
                <Field
                  label={t("stockIncidentalCost")}
                  value={data.stock_incidental_cost ?? ""}
                  inputMode="decimal"
                  disabled={disabled}
                  onChange={(e) =>
                    setData({
                      ...data,
                      stock_incidental_cost: e.target.value.trim()
                        ? e.target.value.replace(",", ".")
                        : null,
                    })
                  }
                />
                <p className="text-sm text-muted">{t("stockIncidentalHelp")}</p>
              </div>
            )}
            <Field
              label={t("receivedRounding")}
              inputMode="decimal"
              disabled={disabled}
              value={data.rounding}
              onChange={(e) =>
                setData({ ...data, rounding: e.target.value.replace(",", ".") })
              }
            />
            <p className="text-lg font-semibold" aria-live="polite">
              {t("receivedTotal")}:{" "}
              {preview
                ? moneyText(preview.total, language, data.currency)
                : !dirty && d
                  ? moneyText(d.totals.total, language, d.data.currency)
                  : "—"}
            </p>
          </div>
          {field("note", "receivedNote")}
          {editing && (
            <div className="flex justify-end gap-control">
              {d && (
                <Button
                  variant="secondary"
                  disabled={busy}
                  onClick={() => {
                    setData(d.data);
                    setEditing(false);
                  }}
                >
                  {t("quoteCancel")}
                </Button>
              )}
              <Button type="submit" disabled={!editable || busy} loading={busy}>
                {t("receivedSave")}
              </Button>
            </div>
          )}
        </div>
      </form>
      {d && (
        <>
          <DocumentAttachments
            company={companyId}
            kind="received"
            id={d.id}
            editable={write && !d.cancelled}
            disabled={busy}
          />
          <section className="invoice-sheet">
            <div className="flex flex-wrap items-center justify-between gap-field p-field">
              <Heading level={2} size="section">
                {t("paymentsTitle")}
              </Heading>
              <p>
                {t("paymentRemaining")}:{" "}
                <strong>
                  {moneyText(d.remaining, language, d.data.currency)}
                </strong>
              </p>
            </div>
            <p className="px-field pb-field text-sm text-muted">
              {t(receipt ? "receiptPaidHint" : "receivedPaymentHint")}
            </p>
            {!d.payments.length ? (
              <p className="p-field text-muted">{t("paymentEmpty")}</p>
            ) : (
              <div
                className="max-w-full overflow-x-auto"
                tabIndex={0}
                role="region"
                aria-label={t("paymentsTitle")}
              >
                <Table className="min-w-2xl">
                  <TableHeader>
                    <TableRow>
                      {[
                        "paymentDate",
                        "paymentAmount",
                        "paymentMethod",
                        "paymentNote",
                        "receivedActions",
                      ].map((k) => (
                        <TableHead key={k}>{t(k as AppKey)}</TableHead>
                      ))}
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {d.payments.map((p) => (
                      <TableRow key={p.id}>
                        <TableCell>
                          {formatCalendarDate(p.paid_on, language)}
                        </TableCell>
                        <TableCell>
                          {moneyText(p.amount, language, data.currency)}
                        </TableCell>
                        <TableCell>
                          {t(
                            p.method === "card"
                              ? "receiptCard"
                              : p.method === "cash"
                                ? "receivedCash"
                                : "receivedTransfer",
                          )}
                        </TableCell>
                        <TableCell>
                          {p.note}
                          {p.reversed_at && (
                            <div>
                              <Badge>{t("paymentReversed")}</Badge>{" "}
                              {p.reason === "receipt_update"
                                ? t("receiptUpdatedPayment")
                                : p.reason}
                            </div>
                          )}
                        </TableCell>
                        <TableCell>
                          {write &&
                            !receipt &&
                            !p.reversed_at &&
                            !d.cancelled && (
                              <Button
                                variant="ghost"
                                disabled={editing || busy}
                                onClick={() => open("reverse", p.id)}
                              >
                                {t("paymentReverse")}
                              </Button>
                            )}
                        </TableCell>
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
            )}
          </section>
        </>
      )}
      <Dialog
        open={!!prompt}
        onClose={() => {
          if (!busy) setPrompt(null);
        }}
        title={prompt ? t(labels[prompt]) : ""}
        description={t(
          prompt === "pay" ? "receivedPaymentHint" : "receivedReversalHint",
        )}
      >
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          {prompt === "pay" && (
            <>
              <Field
                label={t("paymentAmountCurrency", { currency: data.currency })}
                inputMode="decimal"
                required
                value={amount}
                disabled={busy}
                onChange={(e) => setAmount(e.target.value.replace(",", "."))}
              />
              <DatePicker
                label={t("paymentDate")}
                value={date}
                onChange={setDate}
                required
                disabled={busy}
              />
              <Select
                label={t("paymentMethod")}
                value={method}
                disabled={busy}
                onChange={setMethod}
                options={[
                  { value: "bank_transfer", label: t("receivedTransfer") },
                  ...(data.currency === "CZK"
                    ? [{ value: "cash", label: t("receivedCash") }]
                    : []),
                ]}
              />
            </>
          )}
          <Field
            label={t(prompt === "pay" ? "paymentNote" : "paymentReason")}
            value={reason}
            required={prompt !== "pay"}
            disabled={busy}
            onChange={(e) => setReason(e.target.value)}
          />
          {error && (
            <Alert tone="danger" role="alert">
              {t(error)}
            </Alert>
          )}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setPrompt(null)}
            >
              {t("quoteCancel")}
            </Button>
            <Button type="submit" loading={busy}>
              {prompt ? t(labels[prompt]) : ""}
            </Button>
          </div>
        </form>
      </Dialog>
    </div>
  );
}
