import { useEffect, useRef, useState } from "react";
import {
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  Alert,
  Button,
  DatePicker,
  Dialog,
  Field,
  Heading,
  Select,
  TextareaField,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  getPayments,
  registerPayment,
  reversePayment,
  InvoiceCalculationFailure,
  type Payments,
  type Payment,
  type PaymentInput,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { PaymentStatus } from "../molecules/PaymentStatus";
export function InvoicePayments({
  company,
  id,
  editable,
  onSummary,
  currency = "CZK",
}: {
  currency?: string;
  company: string;
  id: string;
  editable: boolean;
  onSummary?: (value: Payments["summary"]) => void;
}) {
  const { t, language } = useAppTranslation();
  const { onExpired } = useWorkspace();
  const [value, setValue] = useState<Payments | null>(null),
    [attempt, setAttempt] = useState(0),
    [loading, setLoading] = useState(true),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<AppKey | "">("");
  const [dialog, setDialog] = useState<"add" | "correct" | "reverse" | null>(
      null,
    ),
    [selected, setSelected] = useState<Payment | null>(null);
  const [input, setInput] = useState<PaymentInput>({
      amount: "",
      paid_on: "",
      method: "bank_transfer",
      note: "",
    }),
    [reason, setReason] = useState("");
  useEffect(() => {
    if (value) onSummary?.(value.summary);
  }, [value, onSummary]);
  const request = useRef({ key: crypto.randomUUID(), input: "" });
  const report = (e: unknown) => {
    if (e instanceof InvoiceCalculationFailure && e.status === 401) {
      onExpired();
      return;
    }
    setError(
      e instanceof InvoiceCalculationFailure
        ? e.fields.includes("cash_automatic_date")
          ? "cashAutomaticDate"
          : e.fields.includes("cash_link")
            ? "cashLinkedPayment"
            : e.fields.includes("bank_link")
              ? "bankLinkedPayment"
              : e.status === 409
                ? "paymentConflict"
                : e.status === 422
                  ? "paymentInvalid"
                  : e.status === 403
                    ? "companyForbidden"
                    : "requestFailed"
        : "requestFailed",
    );
  };
  useEffect(() => {
    const c = new AbortController();
    setLoading(true);
    getPayments(apiUrl, company, id, c.signal)
      .then((v) => {
        if (!c.signal.aborted) {
          setValue(v);
          setError("");
        }
      })
      .catch((e) => {
        if (!c.signal.aborted) report(e);
      })
      .finally(() => {
        if (!c.signal.aborted) setLoading(false);
      });
    return () => c.abort();
  }, [company, id, attempt]);
  useEffect(() => {
    const refresh = () => {
      if (!dialog && !busy) setAttempt((v) => v + 1);
    };
    window.addEventListener("focus", refresh);
    const timer = setInterval(refresh, 60000);
    return () => {
      window.removeEventListener("focus", refresh);
      clearInterval(timer);
    };
  }, [dialog, busy]);
  function open(mode: "add" | "correct" | "reverse", payment?: Payment) {
    if (!value) return;
    setSelected(payment ?? null);
    setDialog(mode);
    setReason("");
    setError("");
    request.current = { key: crypto.randomUUID(), input: "" };
    setInput(
      payment
        ? {
            amount: payment.amount,
            paid_on: payment.paid_on,
            method: payment.method as PaymentInput["method"],
            note: payment.note,
          }
        : {
            amount:
              value.summary.outstanding === "0.00"
                ? ""
                : value.summary.outstanding,
            paid_on: value.summary.today,
            method: "bank_transfer",
            note: "",
          },
    );
  }
  async function submit() {
    if (!value || busy || loading) return;
    setBusy(true);
    setError("");
    const payload = {
      revision: value.summary.revision,
      payment: input,
      reason,
      dialog,
      selected: selected?.id,
    };
    const encoded = JSON.stringify(payload);
    if (request.current.input && request.current.input !== encoded)
      request.current.key = crypto.randomUUID();
    request.current.input = encoded;
    try {
      const next =
        dialog === "add"
          ? await registerPayment(apiUrl, company, id, {
              request_id: request.current.key,
              revision: value.summary.revision,
              payment: input,
            })
          : await reversePayment(apiUrl, company, id, selected!.id, {
              request_id: request.current.key,
              revision: value.summary.revision,
              reason,
              replacement: dialog === "correct" ? input : null,
            });
      setValue(next);
      setDialog(null);
      setAttempt((v) => v + 1);
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  }
  const problem = error && (
    <Alert tone="danger" role="alert">
      {t(error)}
      {error === "paymentConflict" && (
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() => setAttempt((v) => v + 1)}
        >
          {t("paymentRefresh")}
        </Button>
      )}
    </Alert>
  );
  return (
    <section className="workspace-panel" aria-label={t("paymentsTitle")}>
      <div className="panel-toolbar">
        <Heading level={2} size="card">
          {t("paymentsTitle")}
        </Heading>
        {editable && (
          <Button
            disabled={
              loading ||
              !value ||
              busy ||
              (value.summary.status === "paid" && value.summary.paid === "0.00")
            }
            onClick={() => open("add")}
          >
            {t("paymentAdd")}
          </Button>
        )}
      </div>
      <div className="p-field space-y-field">
        {!dialog && problem}
        {loading && !value && <p role="status">{t("draftLoading")}</p>}
        {!value && error && (
          <Button variant="secondary" onClick={() => setAttempt((v) => v + 1)}>
            {t("retry")}
          </Button>
        )}
        {value && (
          <>
            <div className="flex flex-wrap items-center justify-between gap-field">
              <PaymentStatus value={value.summary} />
              <dl className="flex flex-wrap gap-group text-sm tabular-nums">
                {(
                  [
                    ["paymentReceived", value.summary.paid],
                    ["paymentRemaining", value.summary.outstanding],
                    ["paymentOverpayment", value.summary.overpayment],
                  ] as const
                ).map(([label, amount]) => (
                  <div key={label}>
                    <dt className="text-muted">{t(label)}</dt>
                    <dd className="font-semibold">
                      {moneyText(amount, language, currency)}
                    </dd>
                  </div>
                ))}
              </dl>
            </div>
            {value.items.length === 0 ? (
              <p className="text-sm text-muted">
                {t(
                  value.summary.status === "paid" &&
                    value.summary.paid === "0.00"
                    ? "paymentZero"
                    : "paymentEmpty",
                )}
              </p>
            ) : (
              <div
                className="overflow-x-auto"
                role="region"
                aria-label={t("invoicePaymentRecords")}
                tabIndex={0}
              >
                <Table className="min-w-invoice-table">
                  <TableHeader>
                    <TableRow>
                      {(
                        [
                          "paymentDate",
                          "paymentAmount",
                          "paymentMethod",
                          "paymentNote",
                          "invoiceListActions",
                        ] as const
                      ).map((k) => (
                        <TableHead key={k}>
                          {k === "paymentAmount"
                            ? t("paymentAmountCurrency", { currency: currency })
                            : t(k)}
                        </TableHead>
                      ))}
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {value.items.map((p) => (
                      <TableRow key={p.id}>
                        <TableCell className="whitespace-nowrap">
                          {formatCalendarDate(p.paid_on, language)}
                        </TableCell>
                        <TableCell className="font-semibold tabular-nums whitespace-nowrap">
                          {moneyText(p.amount, language, currency)}
                        </TableCell>
                        <TableCell>
                          {t(p.method === "cash" ? "cash" : "bankTransfer")}
                        </TableCell>
                        <TableCell>
                          <p className="whitespace-pre-wrap">{p.note || "—"}</p>
                          {p.reversed_at && (
                            <p className="text-muted">
                              {t(
                                p.replacement_id
                                  ? "paymentCorrected"
                                  : "paymentReversed",
                              )}
                              : {p.reason}
                            </p>
                          )}
                        </TableCell>
                        <TableCell>
                          {editable && !p.reversed_at && (
                            <div className="flex gap-inline">
                              <Button
                                size="sm"
                                variant="ghost"
                                disabled={loading || busy}
                                onClick={() => open("correct", p)}
                              >
                                {t("paymentCorrect")}
                              </Button>
                              <Button
                                size="sm"
                                variant="ghost"
                                disabled={loading || busy}
                                onClick={() => open("reverse", p)}
                              >
                                {t("paymentReverse")}
                              </Button>
                            </div>
                          )}
                        </TableCell>
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
            )}
          </>
        )}
      </div>
      <Dialog
        open={dialog !== null}
        onClose={() => {
          if (!busy) setDialog(null);
        }}
        title={t(
          dialog === "add"
            ? "paymentAdd"
            : dialog === "correct"
              ? "paymentCorrect"
              : "paymentReverse",
        )}
        description={t(
          dialog === "add" ? "paymentAddHelp" : "paymentReverseHelp",
        )}
      >
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          {problem}
          {dialog === "add" && value && (
            <dl className="grid grid-cols-2 gap-field rounded-control bg-canvas p-field text-sm tabular-nums">
              {(
                [
                  ["paymentReceived", value.summary.paid],
                  ["paymentRemaining", value.summary.outstanding],
                ] as const
              ).map(([label, amount]) => (
                <div key={label}>
                  <dt className="text-muted">{t(label)}</dt>
                  <dd className="mt-inline font-semibold">
                    {moneyText(amount, language, currency)}
                  </dd>
                </div>
              ))}
            </dl>
          )}
          {dialog !== "reverse" && (
            <>
              <Field
                label={t("paymentAmountCurrency", { currency: currency })}
                value={input.amount}
                inputMode="decimal"
                required
                disabled={busy}
                onChange={(e) =>
                  setInput({
                    ...input,
                    amount: e.target.value.replace(",", "."),
                  })
                }
              />
              <DatePicker
                label={t("paymentDate")}
                value={input.paid_on}
                disabled={busy}
                onChange={(paid_on) => setInput({ ...input, paid_on })}
              />
              <Select
                label={t("paymentMethod")}
                value={input.method}
                disabled={busy}
                options={[
                  { value: "bank_transfer", label: t("bankTransfer") },
                  ...(currency === "CZK"
                    ? [{ value: "cash", label: t("cash") }]
                    : []),
                ]}
                onChange={(method) =>
                  setInput({
                    ...input,
                    method: method as PaymentInput["method"],
                  })
                }
              />
              <TextareaField
                label={t("paymentNote")}
                value={input.note}
                maxLength={500}
                disabled={busy}
                onChange={(e) => setInput({ ...input, note: e.target.value })}
              />
            </>
          )}
          {dialog !== "add" && (
            <TextareaField
              label={t("paymentReason")}
              value={reason}
              required
              maxLength={500}
              disabled={busy}
              onChange={(e) => setReason(e.target.value)}
            />
          )}
          <div className="flex justify-end gap-control">
            <Button
              type="button"
              variant="secondary"
              disabled={busy}
              onClick={() => setDialog(null)}
            >
              {t("cancelAction")}
            </Button>
            <Button type="submit" disabled={busy || loading} loading={busy}>
              {t("paymentConfirm")}
            </Button>
          </div>
        </form>
      </Dialog>
    </section>
  );
}
