import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Alert,
  Button,
  Heading,
  Input,
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  listTaxReceipts,
  SalesFailure,
  type TaxReceiptPage,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import { salesError } from "../utils/sales";
import { moneyText } from "../utils/invoice";
import { RegisterPagination } from "../components/molecules/RegisterPagination";
export function TaxReceiptsPage() {
  const { companyId = "" } = useParams(),
    { onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams(),
    [input, setInput] = useState(params.get("q") ?? ""),
    [result, setResult] = useState<TaxReceiptPage | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [attempt, setAttempt] = useState(0);
  const query = params.get("q") ?? "",
    page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1)),
    size = [10, 25, 50, 100].includes(Number(params.get("size")))
      ? Number(params.get("size"))
      : 25;
  useEffect(() => {
    setInput(query);
  }, [query, companyId]);
  useEffect(() => {
    const c = new AbortController();
    setResult(null);
    setError("");
    listTaxReceipts(apiUrl, companyId, query, page, size, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setResult(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          if (e instanceof SalesFailure && e.status === 401) onExpired();
          setError(salesError(e));
        }
      });
    return () => c.abort();
  }, [companyId, query, page, size, attempt]);
  const change = (v: Record<string, string>) =>
    setParams({ ...Object.fromEntries(params), page: "1", ...v });
  return (
    <div className="space-y-section">
      <Heading level={1} size="page">
        {t("taxReceipts")}
      </Heading>
      <div className="workspace-panel register-sheet">
        <div className="p-field space-y-field">
          <p className="text-sm text-muted">{t("taxListHelp")}</p>
          <Link
            className="text-sm underline"
            to={`/companies/${companyId}/advances`}
          >
            {t("navAdvances")}
          </Link>
          <form
            className="flex flex-wrap items-center gap-inline"
            onSubmit={(e) => {
              e.preventDefault();
              change({ q: input });
            }}
          >
            <Input
              aria-label={t("taxSearch")}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder={t("taxSearch")}
            />
            <Button type="submit" variant="secondary">
              {t("taxSearchAction")}
            </Button>
          </form>
        </div>
        {error ? (
          <Alert tone="danger" role="alert">
            {t(error)}
            <Button variant="ghost" onClick={() => setAttempt((x) => x + 1)}>
              {t("retry")}
            </Button>
          </Alert>
        ) : !result ? (
          <p className="p-field" role="status">
            {t("salesLoading")}
          </p>
        ) : (
          <>
            <div className="overflow-x-auto">
              <Table className="contact-register-table min-w-invoice-register">
                <TableHeader>
                  <TableRow>
                    {[
                      "invoiceDocumentTab",
                      "draftCustomer",
                      "taxAdvance",
                      "taxIssuedOn",
                      "taxPaidOn",
                      "taxReceived",
                    ].map((k) => (
                      <TableHead key={k}>{t(k as AppKey)}</TableHead>
                    ))}
                  </TableRow>
                </TableHeader>
                <tbody>
                  {result.items.map((d) => (
                    <TableRow key={d.id}>
                      <TableCell>
                        <Link
                          className="font-semibold underline"
                          to={`/companies/${companyId}/tax-receipts/${d.id}`}
                        >
                          {d.number}
                        </Link>
                      </TableCell>
                      <TableCell>{d.data.customer?.name}</TableCell>
                      <TableCell>
                        <Link
                          className="underline"
                          to={`/companies/${companyId}/advances/${d.advance_id}`}
                        >
                          {d.advance_number}
                        </Link>
                      </TableCell>
                      <TableCell>
                        {formatCalendarDate(d.data.issued_on ?? "", language)}
                      </TableCell>
                      <TableCell>
                        {formatCalendarDate(d.data.tax_on ?? "", language)}
                      </TableCell>
                      <TableCell>
                        {moneyText(
                          d.totals.payable,
                          language,
                          d.totals.currency,
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!result.items.length && (
              <p className="p-field text-muted">{t("taxEmpty")}</p>
            )}
            <RegisterPagination
              page={page}
              pageSize={size}
              total={result.total}
              onPage={(p) => change({ page: String(p) })}
              onPageSize={(size) => change({ size })}
            />
          </>
        )}
      </div>
    </div>
  );
}
