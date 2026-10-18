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
  listRecurring,
  SalesFailure,
  type RecurringPage,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import { recurringStates } from "../components/organisms/RecurringEditor";
import { salesError } from "../utils/sales";
export function RecurringListPage() {
  const { companyId = "" } = useParams(),
    { onExpired, company } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "",
    page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1));
  const [input, setInput] = useState(query),
    [result, setResult] = useState<RecurringPage | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [attempt, setAttempt] = useState(0);
  useEffect(() => setInput(query), [query]);
  useEffect(() => {
    const c = new AbortController();
    setResult(null);
    setError("");
    listRecurring(apiUrl, companyId, query, page, c.signal)
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
        {t("navRecurring")}
      </Heading>
      <div className="workspace-panel register-sheet">
        <div className="p-field space-y-field">
          <p className="text-sm text-muted">{t("recListHelp")}</p>
          {company?.role !== "viewer" && (
            <Link
              className="ui-button ui-button-primary ui-button-md"
              to={`/companies/${companyId}/recurring/new`}
            >
              {t("recNew")}
            </Link>
          )}
          <form
            className="flex gap-inline"
            onSubmit={(e) => {
              e.preventDefault();
              setParams({ q: input });
            }}
          >
            <Input
              aria-label={t("recSearch")}
              placeholder={t("recSearch")}
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
                      "recName",
                      "draftCustomer",
                      "recNext",
                      "recGenerated",
                      "recState",
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
                          className="font-medium hover:underline"
                          to={`/companies/${companyId}/recurring/${d.id}`}
                        >
                          {d.data.name}
                        </Link>
                      </TableCell>
                      <TableCell>
                        {d.data.invoice.customer?.name || "—"}
                      </TableCell>
                      <TableCell>
                        {d.next_dates[0]
                          ? formatCalendarDate(d.next_dates[0], language)
                          : "—"}
                      </TableCell>
                      <TableCell className="tabular-nums">
                        {d.generated}
                      </TableCell>
                      <TableCell>
                        <Badge>{t(recurringStates[d.status])}</Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!result.total && <p className="p-field">{t("recEmpty")}</p>}
            <div className="p-field flex items-center justify-between gap-field">
              <span>{t("recTotal", { count: result.total })}</span>
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
