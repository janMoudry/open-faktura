import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Alert,
  Button,
  Heading,
  Input,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  Badge,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  listCredits,
  SalesFailure,
  type CreditPage,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
import { salesError } from "../utils/sales";
export function CreditNotesPage() {
  const { companyId = "" } = useParams(),
    { onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "",
    page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1));
  const [input, setInput] = useState(query),
    [result, setResult] = useState<CreditPage | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [attempt, setAttempt] = useState(0);
  useEffect(() => setInput(query), [query]);
  useEffect(() => {
    const c = new AbortController();
    setResult(null);
    setError("");
    listCredits(apiUrl, companyId, query, page, undefined, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setResult(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          if (e instanceof SalesFailure && e.status === 401) onExpired();
          else setError(salesError(e));
        }
      });
    return () => c.abort();
  }, [companyId, query, page, attempt]);
  return (
    <div className="space-y-section">
      <Heading level={1} size="page">
        {t("navCreditNotes")}
      </Heading>
      <div className="workspace-panel register-sheet">
        <div className="p-field space-y-field">
          <p className="text-sm text-muted">{t("creditListHelp")}</p>
          <Link className="underline" to={`/companies/${companyId}/invoices`}>
            {t("navInvoices")}
          </Link>
          <form
            className="flex gap-inline"
            onSubmit={(e) => {
              e.preventDefault();
              setParams({ q: input });
            }}
          >
            <Input
              aria-label={t("creditSearch")}
              placeholder={t("creditSearch")}
              value={input}
              onChange={(e) => setInput(e.target.value)}
            />
            <Button variant="secondary" type="submit">
              {t("contactSearchAction")}
            </Button>
          </form>
        </div>
        {error && (
          <Alert tone="danger" role="alert">
            {t(error)}
            <Button variant="ghost" onClick={() => setAttempt((v) => v + 1)}>
              {t("retry")}
            </Button>
          </Alert>
        )}
        {!result && !error && (
          <p className="p-field" role="status">
            {t("salesLoading")}
          </p>
        )}
        {result && (
          <>
            <div className="overflow-x-auto">
              <Table className="whitespace-nowrap">
                <TableHeader>
                  <TableRow>
                    {[
                      "creditDocument",
                      "creditSource",
                      "draftCustomer",
                      "taxIssuedOn",
                      "creditAmount",
                      "creditRemaining",
                      "creditStatus",
                    ].map((k) => (
                      <TableHead
                        key={k}
                        className={
                          k === "creditAmount" || k === "creditRemaining"
                            ? "text-right"
                            : undefined
                        }
                      >
                        {t(k as AppKey)}
                      </TableHead>
                    ))}
                  </TableRow>
                </TableHeader>
                <tbody>
                  {result.items.map((d) => (
                    <TableRow key={d.id}>
                      <TableCell>
                        <Link
                          className="font-medium hover:underline"
                          to={`/companies/${companyId}/credit-notes/${d.id}`}
                        >
                          {d.number ?? t("creditDraft")}
                        </Link>
                      </TableCell>
                      <TableCell>
                        <Link
                          className="underline"
                          to={`/companies/${companyId}/invoices/${d.invoice_id}`}
                        >
                          {d.invoice_number}
                        </Link>
                      </TableCell>
                      <TableCell>{d.source.data.customer?.name}</TableCell>
                      <TableCell>
                        {formatCalendarDate(d.data.issued_on, language)}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {moneyText(d.totals.payable, language)}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {d.status === "issued"
                          ? moneyText(d.remaining, language)
                          : "—"}
                      </TableCell>
                      <TableCell>
                        <Badge>
                          {t(
                            d.status === "draft"
                              ? "creditDraft"
                              : d.status === "cancelled"
                                ? "creditCancelled"
                                : "creditIssued",
                          )}
                        </Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!result.total && <p className="p-field">{t("creditEmpty")}</p>}
            <div className="p-field flex items-center justify-between gap-field">
              <span>{t("creditCount", { count: result.total })}</span>
              <div className="flex gap-inline">
                <Button
                  variant="secondary"
                  disabled={page === 1}
                  onClick={() =>
                    setParams({ q: query, page: String(page - 1) })
                  }
                >
                  {t("contactPrevious")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={page * 25 >= result.total}
                  onClick={() =>
                    setParams({ q: query, page: String(page + 1) })
                  }
                >
                  {t("contactNext")}
                </Button>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
