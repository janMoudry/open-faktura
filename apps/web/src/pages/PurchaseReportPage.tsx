import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
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
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  getExpenseOverview,
  getExpensePayments,
  SalesFailure,
  type ExpensePage,
  type ExpenseOverview,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";

export function PurchaseReportPage({
  view,
}: {
  view: "payments" | "overview";
}) {
  const { companyId = "" } = useParams();
  return (
    <PurchaseReport key={companyId + view} companyId={companyId} view={view} />
  );
}
function PurchaseReport({
  companyId,
  view,
}: {
  companyId: string;
  view: "payments" | "overview";
}) {
  const { onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const today = new Intl.DateTimeFormat("sv-SE", {
    timeZone: "Europe/Prague",
  }).format(new Date());
  const from = params.get("from") ?? today.slice(0, 7) + "-01",
    to = params.get("to") ?? today;
  const query = params.get("query") ?? "",
    method = params.get("method") ?? "all",
    status = params.get("status") ?? "active";
  const page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1));
  const [filters, setFilters] = useState({ from, to, query, method, status });
  const [payments, setPayments] = useState<ExpensePage | null>(null),
    [overview, setOverview] = useState<ExpenseOverview | null>(null);
  const [error, setError] = useState<AppKey | "">(""),
    [attempt, setAttempt] = useState(0);
  useEffect(
    () => setFilters({ from, to, query, method, status }),
    [from, to, query, method, status],
  );
  useEffect(() => {
    const c = new AbortController();
    setError("");
    setPayments(null);
    setOverview(null);
    if (
      !/^\d{4}-\d{2}-\d{2}$/.test(from) ||
      !/^\d{4}-\d{2}-\d{2}$/.test(to) ||
      from > to
    ) {
      setError("expenseInvalidPeriod");
      return;
    }
    const work =
      view === "overview"
        ? getExpenseOverview(apiUrl, companyId, from, to, c.signal).then(
            (v) => {
              if (!c.signal.aborted) setOverview(v);
            },
          )
        : getExpensePayments(
            apiUrl,
            companyId,
            { from, to, query, method, status, page: String(page) },
            c.signal,
          ).then((v) => {
            if (!c.signal.aborted) setPayments(v);
          });
    work.catch((e) => {
      if (!c.signal.aborted) {
        if (e instanceof SalesFailure && e.status === 401) onExpired();
        setError(
          e instanceof SalesFailure && (e.status === 400 || e.status === 422)
            ? "expenseInvalidPeriod"
            : "requestFailed",
        );
      }
    });
    return () => c.abort();
  }, [companyId, view, from, to, query, method, status, page, attempt]);
  const title = view === "overview" ? "expenseOverview" : "expensePayments";
  return (
    <div className="space-y-group">
      <Heading level={1} size="page">
        {t(title)}
      </Heading>
      <section className="workspace-panel">
        <form
          className="panel-toolbar flex flex-wrap items-end gap-field"
          onSubmit={(e) => {
            e.preventDefault();
            setParams(
              view === "overview"
                ? { from: filters.from, to: filters.to }
                : filters,
            );
          }}
        >
          <DatePicker
            label={t("expenseFrom")}
            value={filters.from}
            required
            onChange={(v) => setFilters({ ...filters, from: v })}
          />
          <DatePicker
            label={t("expenseTo")}
            value={filters.to}
            required
            onChange={(v) => setFilters({ ...filters, to: v })}
          />
          {view === "payments" && (
            <>
              <Field
                label={t("expenseSearch")}
                value={filters.query}
                onChange={(e) =>
                  setFilters({ ...filters, query: e.target.value })
                }
              />
              <Select
                label={t("paymentMethod")}
                value={filters.method}
                onChange={(v) => setFilters({ ...filters, method: v })}
                options={[
                  { value: "all", label: t("expenseAllMethods") },
                  { value: "cash", label: t("receivedCash") },
                  { value: "card", label: t("receiptCard") },
                  { value: "bank_transfer", label: t("receivedTransfer") },
                ]}
              />
              <Select
                label={t("receivedFilter")}
                value={filters.status}
                onChange={(v) => setFilters({ ...filters, status: v })}
                options={[
                  { value: "active", label: t("expenseActive") },
                  { value: "reversed", label: t("paymentReversed") },
                  { value: "all", label: t("expenseAllPayments") },
                ]}
              />
            </>
          )}
          <Button type="submit" variant="secondary">
            {t("expenseApply")}
          </Button>
        </form>
        {error ? (
          <Alert role="alert" tone="danger">
            {t(error)}{" "}
            <Button onClick={() => setAttempt((v) => v + 1)}>
              {t("retry")}
            </Button>
          </Alert>
        ) : !overview && !payments ? (
          <p className="p-field" role="status">
            {t("draftLoading")}
          </p>
        ) : overview ? (
          <div className="p-field space-y-group">
            <div className="grid gap-field sm:grid-cols-2 xl:grid-cols-4">
              {(
                ["purchases", "expenses", "outstanding", "overdue"] as const
              ).map((k) => (
                <section
                  className="rounded-panel border border-border p-field space-y-control"
                  key={k}
                >
                  <Heading level={2} size="card">
                    {t(
                      (
                        {
                          purchases: "expensePurchases",
                          expenses: "expensePaid",
                          outstanding: "expenseOutstanding",
                          overdue: "expenseOverdue",
                        } as const
                      )[k],
                    )}
                  </Heading>
                  <p className="text-2xl font-semibold tabular-nums">
                    {moneyText(overview[k], language)}
                  </p>
                  <p className="text-sm text-muted">
                    {t(
                      (
                        {
                          purchases: "expensePurchasesHint",
                          expenses: "expensePaidHint",
                          outstanding: "expenseCurrentHint",
                          overdue: "expenseOverdueHint",
                        } as const
                      )[k],
                    )}
                  </p>
                </section>
              ))}
            </div>
            <p className="text-sm text-muted">{t("expenseOverviewHint")}</p>
            <Link
              className="ui-button ui-button-secondary ui-button-md"
              to={`/companies/${companyId}/expense-payments?from=${from}&to=${to}`}
            >
              {t("expensePayments")}
            </Link>
          </div>
        ) : (
          payments && (
            <>
              <div
                className="max-w-full overflow-x-auto"
                role="region"
                tabIndex={0}
                aria-label={t(title)}
              >
                <Table className="min-w-2xl">
                  <TableHeader>
                    <TableRow>
                      {(
                        [
                          "paymentDate",
                          "receivedNumber",
                          "draftSupplier",
                          "receivedDescription",
                          "paymentMethod",
                          "paymentAmount",
                        ] as const
                      ).map((k) => (
                        <TableHead key={k}>{t(k)}</TableHead>
                      ))}
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {payments.items.map((v) => (
                      <TableRow key={v.document_id + ":" + v.payment.id}>
                        <TableCell>
                          {formatCalendarDate(v.payment.paid_on, language)}
                        </TableCell>
                        <TableCell>
                          <Link
                            className="font-semibold underline"
                            to={`/companies/${companyId}/${v.kind === "receipt" ? "receipts" : "received-invoices"}/${v.document_id}`}
                          >
                            {v.number}
                          </Link>
                          <div className="text-sm text-muted">
                            {t(
                              v.kind === "receipt"
                                ? "receiptSingle"
                                : "receivedSingle",
                            )}
                          </div>
                          {v.payment.reversed_at && (
                            <Badge>{t("paymentReversed")}</Badge>
                          )}
                        </TableCell>
                        <TableCell>{v.supplier || "—"}</TableCell>
                        <TableCell>
                          {v.description}
                          <div className="text-sm text-muted">
                            {v.payment.note}
                          </div>
                        </TableCell>
                        <TableCell>
                          {t(
                            v.payment.method === "card"
                              ? "receiptCard"
                              : v.payment.method === "cash"
                                ? "receivedCash"
                                : "receivedTransfer",
                          )}
                        </TableCell>
                        <TableCell className="text-right whitespace-nowrap tabular-nums">
                          {moneyText(v.payment.amount, language, v.currency)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
              {!payments.items.length && (
                <p className="p-field text-muted">{t("expenseEmpty")}</p>
              )}
              <div className="panel-footer flex items-center justify-end gap-control">
                <span>{t("quotePage", { page })}</span>
                <Button
                  disabled={page === 1}
                  onClick={() =>
                    setParams({
                      from,
                      to,
                      query,
                      method,
                      status,
                      page: String(page - 1),
                    })
                  }
                >
                  {t("quotePrevious")}
                </Button>
                <Button
                  disabled={page * 25 >= payments.total}
                  onClick={() =>
                    setParams({
                      from,
                      to,
                      query,
                      method,
                      status,
                      page: String(page + 1),
                    })
                  }
                >
                  {t("quoteNext")}
                </Button>
              </div>
            </>
          )
        )}
      </section>
    </div>
  );
}
