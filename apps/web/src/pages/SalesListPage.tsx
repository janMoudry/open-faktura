import { advanceStatus } from "../utils/advance";
import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  Alert,
  Button,
  Input,
  FilterTabs,
  Heading,
  Icon,
  Select,
  Badge,
} from "@open-faktura/ui";
import {
  listSales,
  SalesFailure,
  type SalesKind,
  type SalesPage,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
import { salesError, salesStates } from "../utils/sales";
import { RegisterPagination } from "../components/molecules/RegisterPagination";
export function SalesListPage({ kind }: { kind: SalesKind }) {
  const { companyId = "" } = useParams();
  const { company, onExpired } = useWorkspace();
  const { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "";
  const status = params.get("status") ?? "all";
  const archived = params.get("archived") === "true";
  const page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1));
  const size = [10, 25, 50, 100].includes(Number(params.get("size")))
    ? Number(params.get("size"))
    : 25;
  const [input, setInput] = useState(query);
  const [result, setResult] = useState<SalesPage | null>(null);
  const [error, setError] = useState<AppKey | "">("");
  const [attempt, setAttempt] = useState(0);
  const [loaded, setLoaded] = useState("");
  const key = JSON.stringify([
    companyId,
    kind,
    query,
    status,
    archived,
    page,
    size,
    attempt,
  ]);
  const base = `/companies/${companyId}/${kind === "advance" ? "advances" : kind === "quote" ? "quotes" : "templates"}`;
  const change = (v: Record<string, string>) =>
    setParams({ ...Object.fromEntries(params), page: "1", ...v });
  useEffect(() => setInput(query), [query, companyId, kind]);
  useEffect(() => {
    const controller = new AbortController();
    setError("");
    listSales(
      apiUrl,
      companyId,
      {
        kind,
        query,
        status: kind !== "template" ? status : "all",
        archived,
        page,
        page_size: size,
      },
      controller.signal,
    )
      .then((v) => {
        if (!controller.signal.aborted) {
          setResult(v);
          setLoaded(key);
        }
      })
      .catch((e) => {
        if (!controller.signal.aborted) {
          if (e instanceof SalesFailure && e.status === 401) onExpired();
          setError(salesError(e));
          setLoaded(key);
        }
      });
    return () => controller.abort();
  }, [companyId, kind, query, status, archived, page, size, key, onExpired]);
  return (
    <div className="space-y-group">
      <Heading level={1} size="page">
        {t(
          kind === "advance"
            ? "navAdvances"
            : kind === "quote"
              ? "salesQuotes"
              : "salesTemplates",
        )}
      </Heading>
      <div className="workspace-panel register-sheet">
        <div className="invoice-list-tabs">
          <FilterTabs
            label={t(kind === "advance" ? "advanceStatusLabel" : "salesStatus")}
            value={archived ? "archived" : "active"}
            options={[
              { value: "active", label: t("salesActive") },
              { value: "archived", label: t("salesArchived") },
            ]}
            onChange={(v) => change({ archived: String(v === "archived") })}
          />
        </div>
        <div className="p-field border-b border-border flex flex-wrap items-end justify-between gap-field">
          <form
            className="flex gap-inline"
            onSubmit={(e) => {
              e.preventDefault();
              change({ q: input });
            }}
          >
            <Input
              aria-label={t("salesSearch")}
              placeholder={t("salesSearch")}
              value={input}
              onChange={(e) => setInput(e.target.value)}
            />
            <Button
              variant="secondary"
              type="submit"
              aria-label={t("contactSearchAction")}
            >
              <Icon name="search" />
            </Button>
          </form>
          <div className="flex flex-wrap items-end gap-field">
            {kind !== "template" && (
              <Select
                label={t(
                  kind === "advance" ? "advanceStatusLabel" : "salesStatus",
                )}
                value={status}
                options={[
                  { value: "all", label: t("salesAll") },
                  ...Object.entries(
                    kind === "advance"
                      ? ({
                          paid: "paymentPaid",
                          outstanding: "paymentOutstanding",
                          overdue: "paymentOverdue",
                        } as const)
                      : salesStates,
                  ).map(([value, label]) => ({
                    value,
                    label: t(label),
                  })),
                ]}
                onChange={(v) => change({ status: v })}
              />
            )}{" "}
            {company?.role !== "viewer" && (
              <Link
                className="ui-button ui-button-primary ui-button-md"
                to={`${base}/new`}
              >
                <Icon name="plus" />
                {t(
                  kind === "advance"
                    ? "advanceNew"
                    : kind === "quote"
                      ? "salesNewQuote"
                      : "salesNewTemplate",
                )}
              </Link>
            )}
          </div>
        </div>
        {error ? (
          <Alert tone="danger" role="alert">
            {t(error)}{" "}
            <Button onClick={() => setAttempt((v) => v + 1)}>
              {t("retry")}
            </Button>
          </Alert>
        ) : loaded !== key ? (
          <p className="p-field" role="status">
            {t("salesLoading")}
          </p>
        ) : (
          result && (
            <>
              <div
                className="overflow-x-auto"
                tabIndex={0}
                role="region"
                aria-label={t(
                  kind === "advance"
                    ? "navAdvances"
                    : kind === "quote"
                      ? "salesQuotes"
                      : "salesTemplates",
                )}
              >
                <Table className="contact-register-table min-w-invoice-register">
                  <TableHeader>
                    <TableRow>
                      <TableHead>
                        {t(
                          kind === "template"
                            ? "salesName"
                            : "invoiceDocumentTab",
                        )}
                      </TableHead>
                      {kind !== "template" && (
                        <TableHead>
                          {t(
                            kind === "advance"
                              ? "advanceStatusLabel"
                              : "salesStatus",
                          )}
                        </TableHead>
                      )}
                      <TableHead>{t("draftCustomer")}</TableHead>
                      {kind !== "template" && (
                        <>
                          <TableHead>{t("draftIssued")}</TableHead>
                          <TableHead>
                            {t(
                              kind === "advance"
                                ? "draftDue"
                                : "salesValidUntil",
                            )}
                          </TableHead>
                        </>
                      )}
                      <TableHead className="text-right">
                        {t("invoiceColumnAmount")}
                      </TableHead>
                      <TableHead>{t("invoiceListActions")}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {result.items.map((d) => (
                      <TableRow key={d.id}>
                        <TableCell>
                          <Link
                            className="font-medium hover:underline"
                            to={`${base}/${d.id}`}
                          >
                            {d.number ?? d.data.name}
                          </Link>
                          {d.number && (
                            <div className="text-xs text-muted">
                              {d.data.name}
                            </div>
                          )}
                        </TableCell>
                        {kind !== "template" && (
                          <TableCell>
                            <Badge>
                              {t(
                                kind === "advance"
                                  ? d.invoice_id
                                    ? d.data.status === "invoiced"
                                      ? "settlementCompleted"
                                      : "settlementReserved"
                                    : advanceStatus(d)
                                  : (salesStates[d.data.status] ??
                                      "salesCreated"),
                              )}
                            </Badge>
                          </TableCell>
                        )}
                        <TableCell>
                          {d.data.invoice.customer?.name || "—"}
                        </TableCell>
                        {kind !== "template" && (
                          <>
                            <TableCell>
                              {d.data.invoice.issued_on
                                ? new Intl.DateTimeFormat(language).format(
                                    new Date(
                                      d.data.invoice.issued_on + "T12:00:00Z",
                                    ),
                                  )
                                : "—"}
                            </TableCell>
                            <TableCell>
                              {d.data.invoice.due_on
                                ? new Intl.DateTimeFormat(language).format(
                                    new Date(
                                      d.data.invoice.due_on + "T12:00:00Z",
                                    ),
                                  )
                                : "—"}
                            </TableCell>
                          </>
                        )}
                        <TableCell className="text-right tabular-nums">
                          {d.totals
                            ? moneyText(d.totals.payable, language)
                            : "—"}
                        </TableCell>
                        <TableCell>
                          <Link
                            className="ui-button ui-button-ghost ui-button-sm"
                            aria-label={`${t("salesOpen")} ${d.number ?? d.data.name}`}
                            to={`${base}/${d.id}`}
                          >
                            <Icon name="chevronRight" />
                          </Link>
                        </TableCell>
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
              {!result.items.length && (
                <p className="p-group text-muted">{t("salesEmpty")}</p>
              )}
              <RegisterPagination
                page={page}
                pageSize={size}
                total={result.total}
                onPage={(v) => change({ page: String(v) })}
                onPageSize={(v) => change({ size: v })}
              />
            </>
          )
        )}
      </div>
    </div>
  );
}
