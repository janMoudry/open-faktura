import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Alert,
  Button,
  Heading,
  Input,
  Select,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  Badge,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  listReceived,
  SalesFailure,
  type ReceivedPage,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation } from "../locales";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
export function ReceivedInvoicesPage({
  kind = "invoice",
}: {
  kind?: "invoice" | "receipt";
}) {
  const { companyId = "" } = useParams(),
    { company, onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "",
    status = params.get("status") ?? "all",
    page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1));
  const [input, setInput] = useState(query),
    [result, setResult] = useState<ReceivedPage | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0);
  useEffect(() => setInput(query), [query]);
  useEffect(() => {
    const c = new AbortController();
    setError(false);
    setResult(null);
    listReceived(apiUrl, companyId, query, status, page, c.signal, kind)
      .then((v) => {
        if (!c.signal.aborted) setResult(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, query, status, page, kind, attempt]);
  return (
    <div className="space-y-group">
      <div className="flex flex-wrap justify-between items-center gap-field">
        <Heading level={1} size="page">
          {t(kind === "receipt" ? "receiptsTitle" : "receivedTitle")}
        </Heading>
        {company?.id === companyId && company.role !== "viewer" && (
          <Link className="ui-button ui-button-primary ui-button-md" to="new">
            {t(kind === "receipt" ? "receiptsNew" : "receivedNew")}
          </Link>
        )}
      </div>
      <div className="workspace-panel register-sheet">
        <form
          className="flex flex-wrap gap-field items-end p-field"
          onSubmit={(e) => {
            e.preventDefault();
            setParams({ q: input, status });
          }}
        >
          <Input
            aria-label={t("receivedSearch")}
            placeholder={t("receivedSearch")}
            value={input}
            onChange={(e) => setInput(e.target.value)}
          />
          <Button type="submit" variant="secondary">
            {t("receivedSearchAction")}
          </Button>
          <Select
            label={t("receivedFilter")}
            value={status}
            onChange={(v) => setParams({ q: query, status: v })}
            options={[
              { value: "all", label: t("receivedAll") },
              ...(kind === "invoice"
                ? [
                    { value: "unpaid", label: t("paymentUnpaid") },
                    { value: "overdue", label: t("paymentOverdue") },
                  ]
                : []),
              { value: "paid", label: t("paymentPaid") },
              { value: "cancelled", label: t("receivedCancelled") },
            ]}
          />
        </form>
        {error ? (
          <Alert tone="danger" role="alert">
            {t("requestFailed")}{" "}
            <Button onClick={() => setAttempt((v) => v + 1)}>
              {t("retry")}
            </Button>
          </Alert>
        ) : !result ? (
          <p className="p-field" role="status">
            {t("draftLoading")}
          </p>
        ) : (
          <>
            <div
              className="max-w-full overflow-x-auto"
              tabIndex={0}
              role="region"
              aria-label={t(
                kind === "receipt" ? "receiptsTitle" : "receivedTitle",
              )}
            >
              <Table className="min-w-2xl">
                <TableHeader>
                  <TableRow>
                    {[
                      "receivedNumber",
                      "draftSupplier",
                      kind === "receipt" ? "draftIssued" : "receivedReceived",
                      ...(kind === "invoice" ? ["draftDue"] : []),
                      "receivedTotal",
                      ...(kind === "invoice" ? ["paymentRemaining"] : []),
                    ].map((k) => (
                      <TableHead key={k}>{t(k as "receivedNumber")}</TableHead>
                    ))}
                  </TableRow>
                </TableHeader>
                <tbody>
                  {result.items.map((d) => (
                    <TableRow key={d.id}>
                      <TableCell>
                        <Link className="font-semibold underline" to={d.id}>
                          {d.number}
                        </Link>
                        <div className="text-sm text-muted">
                          {d.data.description || d.data.lines[0]?.description}
                          {d.data.external_number && (
                            <div>{d.data.external_number}</div>
                          )}
                        </div>
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
                      </TableCell>
                      <TableCell>{d.data.supplier.name || "—"}</TableCell>
                      <TableCell>
                        {formatCalendarDate(d.data.received_on, language)}
                      </TableCell>
                      {kind === "invoice" && (
                        <TableCell>
                          {formatCalendarDate(d.data.due_on, language)}
                        </TableCell>
                      )}
                      <TableCell className="text-right whitespace-nowrap">
                        {moneyText(d.totals.total, language)}
                      </TableCell>
                      {kind === "invoice" && (
                        <TableCell className="text-right whitespace-nowrap">
                          {moneyText(d.remaining, language)}
                        </TableCell>
                      )}
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!result.items.length && (
              <p className="p-field text-muted">{t("receivedEmpty")}</p>
            )}
            <div className="flex items-center justify-end gap-control p-field">
              <span>{t("quotePage", { page })}</span>
              <Button
                disabled={page === 1}
                onClick={() =>
                  setParams({ q: query, status, page: String(page - 1) })
                }
              >
                {t("quotePrevious")}
              </Button>
              <Button
                disabled={page * 25 >= result.total}
                onClick={() =>
                  setParams({ q: query, status, page: String(page + 1) })
                }
              >
                {t("quoteNext")}
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
