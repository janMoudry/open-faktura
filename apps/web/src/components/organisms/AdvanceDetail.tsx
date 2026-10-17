import { AdvanceSettlementPicker } from "./AdvanceSettlementPicker";
import { Link } from "react-router";
import { TaxReceiptIssue } from "./TaxReceiptIssue";
import { useEffect, useRef, useState } from "react";
import {
  formatCalendarDate,
  Alert,
  Badge,
  Button,
  DatePicker,
  Dialog,
  Field,
  Select,
  Tabs,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  advancePdf,
  advancePayments,
  payAdvance,
  reverseAdvance,
  SalesFailure,
  type SalesDocument,
  type AdvanceReceipt,
} from "@open-faktura/api-client";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
import { salesError } from "../../utils/sales";
import { moneyText } from "../../utils/invoice";
import { advanceStatus } from "../../utils/advance";
import { SalesOverview } from "./SalesOverview";
export function AdvanceDetail({
  document: d,
  onChanged,
}: {
  document: SalesDocument;
  onChanged: () => void;
}) {
  const { company, onExpired } = useWorkspace();
  const { t, language } = useAppTranslation();
  const [receipts, setReceipts] = useState<AdvanceReceipt[] | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [busy, setBusy] = useState(false),
    [open, setOpen] = useState(false),
    [reverse, setReverse] = useState<string | null>(null);
  const [amount, setAmount] = useState(""),
    [date, setDate] = useState(
      new Intl.DateTimeFormat("en-CA", {
        timeZone: "Europe/Prague",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
      }).format(new Date()),
    ),
    [method, setMethod] = useState("bank_transfer"),
    [note, setNote] = useState(""),
    [reason, setReason] = useState("");
  const [settling, setSettling] = useState(false);
  const [taxPayment, setTaxPayment] = useState<string | null>(null);
  const key = useRef(crypto.randomUUID());
  const last = useRef("");
  const canWrite = company?.role !== "viewer" && !d.archived && !d.invoice_id;
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else setError(salesError(e));
  }
  useEffect(() => {
    let active = true;
    setReceipts(null);
    advancePayments(apiUrl, d.company_id, d.id)
      .then((v) => {
        if (active) setReceipts(v);
      })
      .catch((e) => {
        if (active) failure(e);
      });
    return () => {
      active = false;
    };
  }, [d.id, d.company_id, d.revision]);
  async function pdf() {
    setBusy(true);
    setError("");
    try {
      const blob = await advancePdf(apiUrl, d.company_id, d.id);
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${d.number}.pdf`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  async function save() {
    setBusy(true);
    setError("");
    try {
      if (reverse) {
        await reverseAdvance(apiUrl, d.company_id, d.id, reverse, {
          revision: d.revision,
          reason,
        });
      } else {
        const payment = {
          amount: amount.replace(",", "."),
          paid_on: date,
          method: method as "bank_transfer" | "cash",
          note,
        };
        const hash = JSON.stringify([d.revision, payment]);
        if (last.current && last.current !== hash)
          key.current = crypto.randomUUID();
        last.current = hash;
        await payAdvance(apiUrl, d.company_id, d.id, {
          request_id: key.current,
          revision: d.revision,
          payment,
        });
        key.current = crypto.randomUUID();
        last.current = "";
      }
      setOpen(false);
      setReverse(null);
      setAmount("");
      setNote("");
      setReason("");
      onChanged();
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="space-y-field">
      <div className="flex flex-wrap items-center gap-inline">
        <Badge>{t(advanceStatus(d))}</Badge>
        <span>
          {t("paymentPaid")}: {moneyText(d.advance_paid ?? "0", language)}
        </span>
        <Button variant="secondary" disabled={busy} onClick={() => void pdf()}>
          {t("advancePdf")}
        </Button>
        {d.invoice_id ? (
          <Link
            className="underline"
            to={`/companies/${d.company_id}/invoices/${d.invoice_id}`}
          >
            {t("settlementOpen")}
          </Link>
        ) : (
          <Button
            disabled={!canWrite || advanceStatus(d) !== "paymentPaid"}
            title={t("settlementEligibility")}
            onClick={() => setSettling(true)}
          >
            {t("advanceSettle")}
          </Button>
        )}
      </div>
      {error && (
        <Alert tone="danger" role="alert">
          {t(error)}
          <Button variant="ghost" onClick={onChanged}>
            {t("retry")}
          </Button>
        </Alert>
      )}
      <Tabs
        label={t("invoiceDocumentTab")}
        items={[
          {
            id: "document",
            label: t("invoiceDocumentTab"),
            content: <SalesOverview document={d} />,
          },
          {
            id: "payments",
            label: t("advancePayments"),
            content: (
              <div className="space-y-field">
                <p className="text-sm text-muted">{t("advancePaymentHint")}</p>
                {canWrite && (
                  <Button
                    onClick={() => {
                      setReverse(null);
                      setError("");
                      setOpen(true);
                    }}
                  >
                    {t("advancePay")}
                  </Button>
                )}
                {receipts === null ? (
                  <p role="status">{t("salesLoading")}</p>
                ) : (
                  <div className="overflow-x-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>{t("paymentDate")}</TableHead>
                          <TableHead>{t("paymentAmount")}</TableHead>
                          <TableHead>{t("advanceStatusLabel")}</TableHead>
                          <TableHead>{t("invoiceListActions")}</TableHead>
                        </TableRow>
                      </TableHeader>
                      <tbody>
                        {receipts.map((p) => (
                          <TableRow key={p.id}>
                            <TableCell>
                              {formatCalendarDate(p.paid_on, language)}
                            </TableCell>
                            <TableCell>
                              {moneyText(p.amount, language)}
                            </TableCell>
                            <TableCell>
                              {p.reversed
                                ? t("paymentReversed")
                                : t("paymentPaid")}
                              <div className="text-sm text-muted">
                                {p.reason ?? p.note}
                              </div>
                            </TableCell>
                            <TableCell>
                              {p.tax_receipt_id && (
                                <Link
                                  className="underline"
                                  to={`/companies/${d.company_id}/tax-receipts/${p.tax_receipt_id}`}
                                >
                                  {p.tax_receipt_number}
                                </Link>
                              )}
                              {canWrite &&
                                !p.reversed &&
                                !p.tax_receipt_id &&
                                d.data.invoice.calculation.tax_mode ===
                                  "vat_payer" && (
                                  <Button
                                    variant="ghost"
                                    onClick={() => setTaxPayment(p.id)}
                                  >
                                    {t("taxIssue")}
                                  </Button>
                                )}
                              {canWrite && !p.reversed && !p.tax_receipt_id && (
                                <Button
                                  variant="ghost"
                                  onClick={() => {
                                    setReverse(p.id);
                                    setError("");
                                    setOpen(true);
                                  }}
                                >
                                  {t("paymentReverse")}
                                </Button>
                              )}
                            </TableCell>
                          </TableRow>
                        ))}
                      </tbody>
                    </Table>
                    {receipts.length === 0 && (
                      <p className="p-field text-muted">
                        {t("advanceNoPayments")}
                      </p>
                    )}
                  </div>
                )}
              </div>
            ),
          },
        ]}
      />
      {settling && (
        <AdvanceSettlementPicker
          company={d.company_id}
          initial={d}
          onClose={() => setSettling(false)}
        />
      )}
      {taxPayment && (
        <TaxReceiptIssue
          document={d}
          paymentId={taxPayment}
          onClose={() => setTaxPayment(null)}
        />
      )}
      <Dialog
        open={open}
        onClose={() => {
          if (!busy) setOpen(false);
        }}
        title={t(reverse ? "paymentReverse" : "advancePay")}
        description={t(reverse ? "paymentReverseHelp" : "advancePaymentHint")}
      >
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          {reverse ? (
            <Field
              label={t("paymentReason")}
              required
              value={reason}
              disabled={busy}
              onChange={(e) => setReason(e.target.value)}
            />
          ) : (
            <>
              <Field
                label={t("paymentAmount")}
                required
                inputMode="decimal"
                value={amount}
                disabled={busy}
                onChange={(e) => setAmount(e.target.value)}
              />
              <DatePicker
                label={t("paymentDate")}
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
                  { value: "bank_transfer", label: t("bankTransfer") },
                  { value: "cash", label: t("cash") },
                ]}
              />
              <Field
                label={t("advanceNote")}
                value={note}
                disabled={busy}
                onChange={(e) => setNote(e.target.value)}
              />
            </>
          )}
          {error && (
            <Alert tone="danger" role="alert">
              {t(error)}
            </Alert>
          )}
          <Button disabled={busy} type="submit">
            {t("salesSave")}
          </Button>
        </form>
      </Dialog>
    </div>
  );
}
