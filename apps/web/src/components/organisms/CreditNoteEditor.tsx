import { useEffect, useState } from "react";
import { Link } from "react-router";
import {
  Alert,
  Button,
  Heading,
  Field,
  DatePicker,
  Select,
  Dialog,
  Badge,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import { creditPdf, type CreditCommand } from "@open-faktura/api-client";
import { useCreditNote } from "../../hooks/useCreditNote";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
import { InvoiceSummary } from "./InvoiceSummary";
type Prompt = "issue" | "cancel" | "offset" | "refund" | "reverse";
export function CreditNoteEditor({
  companyId,
  id,
}: {
  companyId: string;
  id: string;
}) {
  const model = useCreditNote(companyId, id),
    { document: d, data, setData, busy, dirty, error, command } = model,
    { company } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [prompt, setPrompt] = useState<Prompt | null>(null),
    [reason, setReason] = useState(""),
    [amount, setAmount] = useState(""),
    [date, setDate] = useState(""),
    [method, setMethod] = useState("bank_transfer"),
    [event, setEvent] = useState(""),
    [pdfBusy, setPdfBusy] = useState(false),
    [preview, setPreview] = useState<string | null>(null);
  useEffect(
    () => () => {
      if (preview) URL.revokeObjectURL(preview);
    },
    [preview],
  );
  const write = company?.role !== "viewer" && company?.id === companyId,
    editable = write && d?.status === "draft";
  const labels: Record<Prompt, AppKey> = {
    issue: "creditIssue",
    cancel: "creditCancel",
    offset: "creditOffsetAction",
    refund: "creditRefundAction",
    reverse: "creditReverse",
  };
  function open(p: Prompt, e = "") {
    setPrompt(p);
    setReason("");
    setAmount(d?.remaining ?? "");
    setDate(
      new Intl.DateTimeFormat("sv-SE", { timeZone: "Europe/Prague" }).format(
        new Date(),
      ),
    );
    setMethod("bank_transfer");
    setEvent(e);
  }
  async function submit() {
    if (!prompt) return;
    const body: Omit<CreditCommand, "request_id" | "revision"> = {
      action: prompt,
    };
    if (prompt === "refund")
      Object.assign(body, { amount, date, method, reason });
    if (prompt === "reverse") Object.assign(body, { event_id: event, reason });
    if (await command(body)) setPrompt(null);
  }
  async function pdf(show: boolean) {
    setPdfBusy(true);
    try {
      const url = URL.createObjectURL(await creditPdf(apiUrl, companyId, id));
      if (show) setPreview(url);
      else {
        const a = document.createElement("a");
        a.href = url;
        a.download = `${d?.number}.pdf`;
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      }
    } catch (e) {
      model.failure(e);
    } finally {
      setPdfBusy(false);
    }
  }
  return (
    <div className="space-y-section">
      <Link
        className="text-sm underline"
        to={`/companies/${companyId}/credit-notes`}
      >
        {t("navCreditNotes")}
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-field">
        <Heading level={1} size="page">
          {d?.number ?? t("creditDraft")}
        </Heading>
        {d && (
          <Badge>
            {t(
              d.status === "draft"
                ? "creditDraft"
                : d.status === "issued"
                  ? "creditIssued"
                  : "creditCancelled",
            )}
          </Badge>
        )}
      </div>
      {error && (
        <Alert tone="danger" role="alert">
          {t(error)}
          {!d && (
            <Button variant="ghost" onClick={model.reload}>
              {t("retry")}
            </Button>
          )}
        </Alert>
      )}
      {!d && !error && <p role="status">{t("salesLoading")}</p>}
      {d && data && (
        <>
          {d.totals.currency !== "CZK" && (
            <p className="text-sm text-muted">{t("creditCurrencyHelp")}</p>
          )}
          {d.status === "issued" && (
            <div className="flex flex-wrap gap-inline">
              <Button
                variant="secondary"
                disabled={pdfBusy}
                onClick={() => void pdf(false)}
              >
                {t("taxPdf")}
              </Button>
              <Button
                variant="secondary"
                disabled={pdfBusy}
                onClick={() => void pdf(true)}
              >
                {t("creditPrint")}
              </Button>
              {write && (
                <>
                  <Button
                    variant="secondary"
                    disabled={busy || d.remaining === "0.00"}
                    onClick={() => open("offset")}
                  >
                    {t("creditOffsetAction")}
                  </Button>
                  <Button
                    disabled={busy || d.remaining === "0.00"}
                    onClick={() => open("refund")}
                  >
                    {t("creditRefundAction")}
                  </Button>
                </>
              )}
            </div>
          )}
          <article className="invoice-sheet">
            <section className="invoice-section space-y-field">
              <dl className="invoice-facts">
                <div>
                  <dt>{t("creditSource")}</dt>
                  <dd>
                    <Link
                      className="underline"
                      to={`/companies/${companyId}/invoices/${d.invoice_id}`}
                    >
                      {d.invoice_number}
                    </Link>
                  </dd>
                </div>
                <div>
                  <dt>{t("taxSupplier")}</dt>
                  <dd>{d.source.data.supplier.name}</dd>
                </div>
                <div>
                  <dt>{t("draftCustomer")}</dt>
                  <dd>{d.source.data.customer?.name}</dd>
                </div>
              </dl>
              {d.source.advances && <Alert>{t("creditAdvanceHelp")}</Alert>}
              {editable ? (
                <Field
                  label={t("creditReason")}
                  value={data.reason}
                  required
                  maxLength={1000}
                  disabled={busy}
                  onChange={(e) => setData({ ...data, reason: e.target.value })}
                />
              ) : (
                <p>
                  <strong>{t("creditReason")}: </strong>
                  {data.reason || "—"}
                </p>
              )}
            </section>
            <section className="invoice-section grid gap-field md:grid-cols-3">
              {(
                [
                  ["issued_on", "taxIssuedOn"],
                  ["tax_on", "creditTaxOn"],
                  ["due_on", "draftDue"],
                ] as const
              ).map(([key, label]) =>
                editable ? (
                  <DatePicker
                    key={key}
                    label={t(label)}
                    value={data[key]}
                    disabled={busy}
                    onChange={(v) => setData({ ...data, [key]: v })}
                  />
                ) : (
                  <div key={key}>
                    <p className="text-sm text-muted">{t(label)}</p>
                    <p>{formatCalendarDate(data[key], language)}</p>
                  </div>
                ),
              )}
            </section>
            <section className="invoice-section space-y-field">
              <Heading level={2} size="section">
                {t("creditLines")}
              </Heading>
              {editable && (
                <p className="text-sm text-muted">{t("creditLinesHelp")}</p>
              )}
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("draftDescription")}</TableHead>
                      <TableHead>{t("taxRate")}</TableHead>
                      <TableHead>
                        {t("creditAmount")} ({d.totals.currency})
                      </TableHead>
                      <TableHead>{t("stockReturnQuantity")}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {data.lines.map((line, index) => {
                      const original = d.source.data.calculation.lines.find(
                        (l) => l.id === line.id,
                      );
                      return (
                        <TableRow key={line.id}>
                          <TableCell>{original?.description}</TableCell>
                          <TableCell className="whitespace-nowrap">
                            {original?.vat_percent} %
                          </TableCell>
                          <TableCell>
                            {editable ? (
                              <Field
                                label={t("creditLineAmount", {
                                  index: index + 1,
                                })}
                                inputMode="decimal"
                                value={line.amount}
                                disabled={busy}
                                onChange={(e) =>
                                  setData({
                                    ...data,
                                    lines: data.lines.map((l) =>
                                      l.id === line.id
                                        ? {
                                            ...l,
                                            amount: e.target.value.replace(
                                              ",",
                                              ".",
                                            ),
                                          }
                                        : l,
                                    ),
                                  })
                                }
                              />
                            ) : (
                              <span className="whitespace-nowrap">
                                −
                                {moneyText(
                                  line.amount,
                                  language,
                                  d.totals.currency,
                                )}
                              </span>
                            )}
                          </TableCell>
                          <TableCell>
                            {original?.catalog_id ? (
                              <Field
                                label={`${t("stockReturnQuantity")} ${index + 1}`}
                                inputMode="decimal"
                                value={line.return_quantity ?? ""}
                                disabled={!editable || busy}
                                onChange={(e) =>
                                  setData({
                                    ...data,
                                    lines: data.lines.map((l) =>
                                      l.id === line.id
                                        ? {
                                            ...l,
                                            return_quantity:
                                              e.target.value.replace(
                                                ",",
                                                ".",
                                              ) || null,
                                          }
                                        : l,
                                    ),
                                  })
                                }
                              />
                            ) : (
                              "—"
                            )}
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </tbody>
                </Table>
              </div>
              {editable && (
                <Select
                  label={t("taxLanguage")}
                  value={data.language}
                  disabled={busy}
                  onChange={(v) =>
                    setData({ ...data, language: v as "cs" | "en" })
                  }
                  options={[
                    { value: "cs", label: t("taxCzech") },
                    { value: "en", label: t("taxEnglish") },
                  ]}
                />
              )}
            </section>
            {dirty ? (
              <section className="invoice-section">
                <Alert tone="warning">{t("creditSaveFirst")}</Alert>
              </section>
            ) : (
              <InvoiceSummary
                calculation={d.source.data.calculation}
                totals={d.totals}
                totalLabel={t("creditAmount")}
              />
            )}
            {editable && (
              <div className="profile-actions">
                <Button
                  variant="ghost"
                  disabled={busy}
                  onClick={() => open("cancel")}
                >
                  {t("creditCancel")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={busy || !dirty}
                  onClick={() => void command({ action: "update", data })}
                >
                  {t("creditSave")}
                </Button>
                <Button
                  disabled={busy || dirty || !data.reason.trim()}
                  onClick={() => open("issue")}
                >
                  {t("creditIssue")}
                </Button>
              </div>
            )}
          </article>
          {d.status === "issued" && (
            <section className="workspace-panel p-field space-y-field">
              <Heading level={2} size="section">
                {t("creditSettlement")}
              </Heading>
              <dl className="invoice-facts">
                {(
                  [
                    ["creditOffset", d.offset],
                    ["creditRefunded", d.refunded],
                    ["creditRemaining", d.remaining],
                  ] as const
                ).map(([label, value]) => (
                  <div key={label}>
                    <dt>{t(label)}</dt>
                    <dd>{moneyText(value, language, d.totals.currency)}</dd>
                  </div>
                ))}
              </dl>
              <p className="text-sm text-muted">{t("creditManualHelp")}</p>
              <div className="overflow-x-auto">
                <Table className="whitespace-nowrap">
                  <TableHeader>
                    <TableRow>
                      {[
                        "taxIssuedOn",
                        "creditEvent",
                        "creditAmount",
                        "creditReason",
                        "invoiceListActions",
                      ].map((k) => (
                        <TableHead key={k}>{t(k as AppKey)}</TableHead>
                      ))}
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {d.events.map((e) => (
                      <TableRow key={e.id}>
                        <TableCell>
                          {formatCalendarDate(e.date, language)}
                        </TableCell>
                        <TableCell>
                          {t(
                            e.kind === "offset"
                              ? "creditOffset"
                              : e.kind === "refund"
                                ? "creditRefunded"
                                : "creditReversed",
                          )}
                        </TableCell>
                        <TableCell>
                          {moneyText(e.amount, language, d.totals.currency)}
                        </TableCell>
                        <TableCell className="whitespace-normal">
                          {e.reason || "—"}
                        </TableCell>
                        <TableCell>
                          {write &&
                            e.kind !== "reversal" &&
                            !d.events.some((r) => r.reverses === e.id) && (
                              <Button
                                variant="ghost"
                                disabled={busy}
                                onClick={() => open("reverse", e.id)}
                              >
                                {t("creditReverse")}
                              </Button>
                            )}
                        </TableCell>
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
              {!d.events.length && <p>{t("creditNoEvents")}</p>}
            </section>
          )}
        </>
      )}
      {prompt && (
        <Dialog
          open
          title={t(labels[prompt])}
          description={t("creditSourceHelp")}
          onClose={() => {
            if (!busy) setPrompt(null);
          }}
        >
          <form
            className="space-y-field"
            onSubmit={(e) => {
              e.preventDefault();
              void submit();
            }}
          >
            <p>
              {t(
                prompt === "issue"
                  ? "creditIssueHelp"
                  : prompt === "offset"
                    ? "creditOffsetHelp"
                    : prompt === "cancel"
                      ? "creditCancelHelp"
                      : prompt === "reverse"
                        ? "creditReverseHelp"
                        : "creditManualHelp",
              )}
            </p>
            {error && (
              <Alert tone="danger" role="alert">
                {t(error)}
              </Alert>
            )}
            {prompt === "refund" && (
              <>
                <Field
                  label={`${t("creditRefundAmount")} (${d?.totals.currency})`}
                  value={amount}
                  inputMode="decimal"
                  disabled={busy}
                  required
                  onChange={(e) => setAmount(e.target.value.replace(",", "."))}
                />
                <DatePicker
                  label={t("creditRefundDate")}
                  value={date}
                  disabled={busy}
                  onChange={setDate}
                />
                <Select
                  label={t("paymentMethod")}
                  value={method}
                  disabled={busy}
                  onChange={setMethod}
                  options={[
                    ...(d?.totals.currency === "CZK"
                      ? [{ value: "cash", label: t("creditCash") }]
                      : []),
                    { value: "bank_transfer", label: t("creditBank") },
                  ]}
                />
              </>
            )}
            {(prompt === "reverse" || prompt === "refund") && (
              <Field
                label={t("creditReason")}
                required
                maxLength={1000}
                value={reason}
                disabled={busy}
                onChange={(e) => setReason(e.target.value)}
              />
            )}
            <Button type="submit" disabled={busy}>
              {t(labels[prompt])}
            </Button>
          </form>
        </Dialog>
      )}
      {preview && (
        <Dialog
          open
          title={t("creditPrint")}
          description={t("creditOpenPdf")}
          onClose={() => setPreview(null)}
        >
          <div className="space-y-field">
            <a
              className="underline"
              href={preview}
              target="_blank"
              rel="noreferrer"
            >
              {t("creditOpenPdf")}
            </a>
            <iframe
              title={t("creditDocument")}
              src={preview}
              className="w-full min-h-96 border border-border rounded-control"
            />
          </div>
        </Dialog>
      )}
    </div>
  );
}
