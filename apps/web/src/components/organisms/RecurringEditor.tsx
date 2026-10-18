import { useState } from "react";
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
  Switch,
  TextareaField,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import type { Company, RecurringData } from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { useRecurring } from "../../hooks/useRecurring";
import { useInvoiceCalculation } from "../../hooks/useInvoiceCalculation";
import { contactParty, emptyParty } from "../../utils/invoice";
import { InvoiceCustomerPicker } from "./InvoiceCustomerPicker";
import { InvoicePartyFields } from "./InvoicePartyFields";
import { InvoiceLines } from "./InvoiceLines";
import { InvoiceSummary } from "./InvoiceSummary";
export const recurringStates: Record<string, AppKey> = {
  paused: "recPaused",
  active: "recActive",
  completed: "recCompleted",
  error: "recError",
};
export function RecurringEditor({
  company,
  id,
  source,
}: {
  company: Company;
  id: string;
  source: string;
}) {
  const { t, language } = useAppTranslation();
  const h = useRecurring(company, id, source);
  const [confirm, setConfirm] = useState(false);
  const d = h.data;
  const readOnly =
    h.busy || company.role === "viewer" || h.record?.status === "active";
  const anchorLocked = readOnly || (h.record?.generated ?? 0) > 0;
  const patch = (v: Partial<RecurringData>) => h.setData({ ...d, ...v });
  const schedule = (v: Partial<RecurringData["schedule"]>) =>
    patch({ schedule: { ...d.schedule, ...v } });
  const invoice = (v: Partial<RecurringData["invoice"]>) =>
    patch({
      invoice: { ...d.invoice, ...v },
      ...(v.calculation?.currency === "CZK"
        ? { rate_policy: "stored" as const }
        : {}),
    });
  const calculation = useInvoiceCalculation(
    company.id,
    d.invoice.calculation,
    h.onExpired,
    !h.loading,
  );
  if (h.loading) return <p role="status">{t("salesLoading")}</p>;
  return (
    <div className="space-y-section">
      <div className="flex flex-wrap items-center justify-between gap-field">
        <div>
          <Link
            className="text-sm underline"
            to={`/companies/${company.id}/recurring`}
          >
            {t("navRecurring")}
          </Link>
          <Heading level={1} size="page">
            {id === "new" ? t("recNew") : d.name}
          </Heading>
        </div>
        <div className="flex flex-wrap gap-inline">
          {h.record && (
            <>
              <Badge>{t(recurringStates[h.record.status])}</Badge>
              <Button
                variant="secondary"
                disabled={h.busy || h.dirty}
                onClick={h.reload}
              >
                {t("recRefresh")}
              </Button>
            </>
          )}
          {h.record &&
            company.role !== "viewer" &&
            (h.record.status === "active" ? (
              <Button
                variant="secondary"
                disabled={h.busy}
                onClick={() => void h.command({ action: "pause" })}
              >
                {t("recPause")}
              </Button>
            ) : (
              <Button
                disabled={h.busy || h.dirty || !h.dates.length}
                onClick={() => setConfirm(true)}
              >
                {t("recActivate")}
              </Button>
            ))}
        </div>
      </div>
      {h.error && (
        <Alert tone="danger" role="alert">
          {t(h.error)}{" "}
          <Button variant="ghost" disabled={h.busy} onClick={h.reload}>
            {t("reloadCompany")}
          </Button>
        </Alert>
      )}
      {h.record?.error_code && (
        <Alert tone="danger">
          {t(
            h.record.error_code === "numbering"
              ? "recNumberingError"
              : h.record.error_code === "tax_mode"
                ? "recTaxError"
                : h.record.error_code === "permission"
                  ? "recPermissionError"
                  : "recStopped",
          )}
        </Alert>
      )}
      <form
        className="invoice-sheet"
        onSubmit={(e) => {
          e.preventDefault();
          void h.command({ action: "save", data: d });
        }}
      >
        <section className="invoice-section space-y-field">
          <Field
            label={t("recName")}
            required
            maxLength={200}
            value={d.name}
            readOnly={readOnly}
            onChange={(e) => patch({ name: e.target.value })}
          />
          <Heading level={2} size="card">
            {t("recSchedule")}
          </Heading>
          <div className="grid gap-field sm:grid-cols-3">
            <Select
              label={t("recFrequency")}
              value={d.schedule.frequency}
              disabled={anchorLocked}
              options={(["day", "week", "month", "year"] as const).map(
                (value) => ({
                  value,
                  label: t(
                    (
                      {
                        day: "recDay",
                        week: "recWeek",
                        month: "recMonth",
                        year: "recYear",
                      } as const
                    )[value],
                  ),
                }),
              )}
              onChange={(v) =>
                schedule({
                  frequency: v as typeof d.schedule.frequency,
                  month_end: v === "month" && d.schedule.month_end,
                })
              }
            />
            <Field
              type="number"
              min={1}
              max={120}
              label={t("recInterval")}
              value={d.schedule.interval}
              readOnly={anchorLocked}
              onChange={(e) => schedule({ interval: Number(e.target.value) })}
            />
            <DatePicker
              label={t("recFirst")}
              value={d.schedule.first_on}
              disabled={anchorLocked}
              onChange={(v) => schedule({ first_on: v })}
            />
            <Field
              label={t("recTimezone")}
              value={d.schedule.timezone}
              readOnly={anchorLocked}
              onChange={(e) => schedule({ timezone: e.target.value })}
            />
            <Select
              label={t("recEnd")}
              disabled={readOnly}
              value={
                d.schedule.count !== null
                  ? "count"
                  : d.schedule.end_on
                    ? "date"
                    : "never"
              }
              options={[
                { value: "never", label: t("recNever") },
                { value: "date", label: t("recByDate") },
                { value: "count", label: t("recByCount") },
              ]}
              onChange={(v) =>
                schedule({
                  end_on: v === "date" ? d.schedule.first_on : null,
                  count:
                    v === "count"
                      ? Math.max(1, (h.record?.generated ?? 0) + 1)
                      : null,
                })
              }
            />
            {d.schedule.end_on !== null && (
              <DatePicker
                label={t("recLast")}
                value={d.schedule.end_on}
                disabled={readOnly}
                onChange={(v) => schedule({ end_on: v })}
              />
            )}
            {d.schedule.count !== null && (
              <Field
                type="number"
                min={1}
                max={10000}
                label={t("recCount")}
                value={d.schedule.count}
                readOnly={readOnly}
                onChange={(e) => schedule({ count: Number(e.target.value) })}
              />
            )}
          </div>
          {d.schedule.frequency === "month" && (
            <Switch
              label={t("recMonthEnd")}
              checked={d.schedule.month_end}
              disabled={anchorLocked}
              onChange={(e) => schedule({ month_end: e.target.checked })}
            />
          )}
          {(h.record?.generated ?? 0) > 0 && (
            <p className="text-sm text-muted">{t("recAnchorLocked")}</p>
          )}
          <div className="rounded-control border border-border p-field space-y-inline">
            <Heading level={3} size="card">
              {t("recUpcoming")}
            </Heading>
            {h.previewLoading ? (
              <p role="status">{t("salesLoading")}</p>
            ) : h.previewError ? (
              <p role="alert">{t("recScheduleInvalid")}</p>
            ) : (
              <p className="text-sm">
                {h.dates
                  .map((x) => formatCalendarDate(x, language))
                  .join(" · ") || t("recNoDates")}
              </p>
            )}
          </div>
        </section>
        <section className="invoice-section space-y-field">
          <Heading level={2} size="card">
            {t("draftCustomer")}
          </Heading>
          {!readOnly && (
            <InvoiceCustomerPicker
              company={company.id}
              disabled={readOnly}
              onExpired={h.onExpired}
              onSelect={(c) =>
                invoice({ customer: contactParty(c), contact_id: c.id })
              }
            />
          )}
          <p className="font-medium">{d.invoice.customer?.name || "—"}</p>
          <details open={!d.invoice.customer?.name}>
            <summary className="cursor-pointer text-sm">
              {t("salesCustomerDetails")}
            </summary>
            <div className="mt-field">
              <InvoicePartyFields
                value={d.invoice.customer ?? emptyParty()}
                disabled={readOnly}
                onChange={(v) => invoice({ customer: v, contact_id: null })}
              />
            </div>
          </details>
          <details>
            <summary className="cursor-pointer text-sm">
              {t("draftSupplier")}
            </summary>
            <div className="mt-field">
              <InvoicePartyFields
                value={d.invoice.supplier}
                disabled={readOnly}
                onChange={(v) => invoice({ supplier: v })}
              />
            </div>
          </details>
        </section>
        <section className="invoice-section">
          <Select
            label={t("taxMode")}
            value={d.invoice.calculation.tax_mode}
            disabled={readOnly}
            options={[
              { value: "non_payer", label: t("nonPayer") },
              { value: "vat_payer", label: t("vatPayer") },
            ]}
            onChange={(v) =>
              invoice({
                calculation: {
                  ...d.invoice.calculation,
                  tax_mode: v as "non_payer" | "vat_payer",
                  lines: d.invoice.calculation.lines.map((l) => ({
                    ...l,
                    vat_percent: v === "non_payer" ? "0" : l.vat_percent,
                  })),
                },
              })
            }
          />
        </section>
        {d.invoice.calculation.currency !== "CZK" && (
          <div className="invoice-section space-y-field">
            <Select
              label={t("recRatePolicy")}
              value={d.rate_policy ?? "stored"}
              disabled={readOnly}
              onChange={(v) =>
                patch({ rate_policy: v as "stored" | "cnb_issue_date" })
              }
              options={[
                { value: "stored", label: t("recRateStored") },
                { value: "cnb_issue_date", label: t("recRateCnb") },
              ]}
            />
            <p className="text-sm text-muted">{t("recRateHint")}</p>
          </div>
        )}
        <InvoiceLines
          value={d.invoice.calculation}
          onChange={(v) => invoice({ calculation: v })}
          disabled={readOnly}
          errors={[]}
          totals={calculation.totals}
        />
        <section className="invoice-section grid gap-field sm:grid-cols-2">
          <Field
            type="number"
            min={0}
            max={365}
            label={t("salesMaturity")}
            value={d.maturity_days}
            readOnly={readOnly}
            onChange={(e) => patch({ maturity_days: Number(e.target.value) })}
          />
          <Select
            label={t("paymentMethod")}
            value={d.invoice.payment.method}
            disabled={readOnly}
            options={[
              { value: "bank_transfer", label: t("bankTransfer") },
              { value: "cash", label: t("cash") },
            ]}
            onChange={(v) =>
              invoice({
                payment: {
                  ...d.invoice.payment,
                  method: v as "cash" | "bank_transfer",
                },
              })
            }
          />
          <Field
            label={t("recBankAccount")}
            value={d.invoice.payment.bank_account}
            readOnly={readOnly}
            onChange={(e) =>
              invoice({
                payment: { ...d.invoice.payment, bank_account: e.target.value },
              })
            }
          />
          <Field
            label={t("recIban")}
            value={d.invoice.payment.iban}
            readOnly={readOnly}
            onChange={(e) =>
              invoice({
                payment: { ...d.invoice.payment, iban: e.target.value },
              })
            }
          />
          <Field
            label={t("recFixedSymbol")}
            value={d.invoice.payment.variable_symbol}
            readOnly={readOnly}
            onChange={(e) =>
              invoice({
                payment: {
                  ...d.invoice.payment,
                  variable_symbol: e.target.value,
                },
              })
            }
          />
          <Field
            label={t("draftOrder")}
            value={d.invoice.order_number}
            readOnly={readOnly}
            onChange={(e) => invoice({ order_number: e.target.value })}
          />
          <Field
            label={t("invoiceDescription")}
            value={d.invoice.note}
            readOnly={readOnly}
            onChange={(e) => invoice({ note: e.target.value })}
          />
          <Select
            label={t("draftLanguage")}
            value={d.invoice.language}
            disabled={readOnly}
            options={[
              { value: "cs", label: "Čeština" },
              { value: "en", label: "English" },
            ]}
            onChange={(v) => invoice({ language: v as "cs" | "en" })}
          />
        </section>
        <section className="invoice-section space-y-field">
          <Heading level={2} size="card">
            {t("recAutomation")}
          </Heading>
          <p className="text-sm text-muted">{t("recAutomationHelp")}</p>
          <div className="flex flex-col items-start gap-field">
            <Switch
              label={t("recAutoIssue")}
              checked={d.auto_issue}
              disabled={readOnly}
              onChange={(e) =>
                patch({
                  auto_issue: e.target.checked,
                  mail: e.target.checked ? d.mail : null,
                })
              }
            />
            <Switch
              label={t("recAutoSend")}
              checked={!!d.mail}
              disabled={readOnly || !d.auto_issue}
              onChange={(e) =>
                patch({
                  mail: e.target.checked
                    ? {
                        recipient: d.invoice.customer?.email ?? "",
                        subject: t("recMailSubject"),
                        body: t("recMailBody"),
                      }
                    : null,
                })
              }
            />
          </div>
          {d.mail && (
            <div className="grid gap-field sm:grid-cols-2">
              <Field
                type="email"
                label={t("recRecipient")}
                value={d.mail.recipient}
                readOnly={readOnly}
                onChange={(e) =>
                  patch({ mail: { ...d.mail!, recipient: e.target.value } })
                }
              />
              <Field
                label={t("recSubject")}
                value={d.mail.subject}
                readOnly={readOnly}
                onChange={(e) =>
                  patch({ mail: { ...d.mail!, subject: e.target.value } })
                }
              />
              <TextareaField
                label={t("recBody")}
                value={d.mail.body}
                readOnly={readOnly}
                onChange={(e) =>
                  patch({ mail: { ...d.mail!, body: e.target.value } })
                }
              />
            </div>
          )}
        </section>
        <InvoiceSummary
          totals={calculation.totals}
          calculation={d.invoice.calculation}
          loading={calculation.loading}
          failed={calculation.failed}
        />
        {company.role !== "viewer" && (
          <div className="invoice-section flex justify-end">
            <Button
              type="submit"
              disabled={readOnly || (!h.dirty && id !== "new")}
            >
              {t("recSave")}
            </Button>
          </div>
        )}
      </form>
      {h.record && (
        <section className="workspace-panel p-field space-y-field">
          <Heading level={2} size="card">
            {t("recHistory")}
          </Heading>
          <p className="text-sm text-muted">
            {t("recHistoryHelp", { count: h.record.generated })}
          </p>
          <div className="overflow-x-auto">
            <Table className="whitespace-nowrap">
              <TableHeader>
                <TableRow>
                  {["recPlanned", "recInvoice", "recDelivery"].map((k) => (
                    <TableHead key={k}>{t(k as AppKey)}</TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <tbody>
                {h.record.occurrences.map((o) => (
                  <TableRow key={o.index}>
                    <TableCell>
                      {formatCalendarDate(o.scheduled_on, language)}
                    </TableCell>
                    <TableCell>
                      <Link
                        className="underline"
                        to={`/companies/${company.id}/invoices/${o.invoice_id}`}
                      >
                        {o.number ?? t("recDraftInvoice")}
                      </Link>
                    </TableCell>
                    <TableCell>
                      {o.error_code
                        ? t("recMailError")
                        : o.delivery_status
                          ? t(
                              (
                                {
                                  pending: "recPending",
                                  queued: "recQueued",
                                  sending: "recSending",
                                  sent: "recSent",
                                  failed: "recFailed",
                                  uncertain: "recUncertain",
                                } as Record<string, AppKey>
                              )[o.delivery_status] ?? "recPending",
                            )
                          : "—"}
                    </TableCell>
                  </TableRow>
                ))}
              </tbody>
            </Table>
          </div>
        </section>
      )}
      <Dialog
        open={confirm}
        onClose={() => setConfirm(false)}
        title={t("recActivate")}
        description={t("recConfirm")}
      >
        <div className="space-y-field">
          <p>
            {t(
              d.mail
                ? "recConfirmEmail"
                : d.auto_issue
                  ? "recConfirmIssue"
                  : "recConfirmDraft",
            )}
          </p>
          <Button
            disabled={h.busy}
            onClick={() =>
              void h.command({ action: "activate" }).then((ok) => {
                if (ok) setConfirm(false);
              })
            }
          >
            {t("recConfirmAction")}
          </Button>
          {h.error && (
            <Alert tone="danger" role="alert">
              {t(h.error)}
            </Alert>
          )}
        </div>
      </Dialog>
    </div>
  );
}
