import { RegisterPagination } from "../components/molecules/RegisterPagination";
import { InvoiceListFilters } from "../components/organisms/InvoiceListFilters";
import { PaymentStatus } from "../components/molecules/PaymentStatus";
import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  Alert,
  Badge,
  Button,
  Input,
  FilterTabs,
  Heading,
  Icon,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  listDrafts,
  InvoiceCalculationFailure,
  type DraftPage,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import { invoiceError, moneyText } from "../utils/invoice";
export function InvoiceDraftListPage() {
  const { companyId = "" } = useParams();
  const { company, onExpired } = useWorkspace();
  const { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "",
    sort = params.get("sort") ?? "number_desc",
    customer = params.get("customer") ?? "";
  const status = (
    [
      "all",
      "draft",
      "issued",
      "outstanding",
      "overdue",
      "paid",
      "partial",
      "overpaid",
    ].includes(params.get("status") ?? "")
      ? params.get("status")
      : "all"
  ) as
    | "all"
    | "draft"
    | "issued"
    | "outstanding"
    | "overdue"
    | "paid"
    | "partial"
    | "overpaid";
  const page = Math.max(
    1,
    Math.min(10000, Math.trunc(Number(params.get("page")) || 1)),
  );
  const pageSize = [10, 25, 50, 100].includes(Number(params.get("page_size")))
    ? Number(params.get("page_size"))
    : 10;
  const date_field =
    params.get("date_field") === "due_on" ? "due_on" : "issued_on";
  const date_from = params.get("date_from") || undefined,
    date_to = params.get("date_to") || undefined;
  const amount_min = params.get("amount_min") || undefined,
    amount_max = params.get("amount_max") || undefined;
  const rangeActive = !!(date_from || date_to || amount_min || amount_max);
  const [input, setInput] = useState(query);
  const [attempt, setAttempt] = useState(0);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [result, setResult] = useState<{
    key: string;
    value: DraftPage;
  } | null>(null);
  const [error, setError] = useState<AppKey | "">("");
  const key = JSON.stringify([
    companyId,
    query,
    sort,
    customer,
    status,
    page,
    attempt,
    pageSize,
    date_field,
    date_from,
    date_to,
    amount_min,
    amount_max,
  ]);
  useEffect(() => setInput(query), [query, companyId]);
  useEffect(() => {
    const c = new AbortController();
    setError("");
    listDrafts(
      apiUrl,
      companyId,
      {
        query,
        page,
        sort,
        status,
        customer_id: customer || undefined,
        page_size: pageSize,
        date_field,
        date_from,
        date_to,
        amount_min,
        amount_max,
      },
      c.signal,
    )
      .then((value) => {
        if (!c.signal.aborted) setResult({ key, value });
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          if (e instanceof InvoiceCalculationFailure && e.status === 401)
            onExpired();
          else setError(invoiceError(e));
        }
      });
    return () => c.abort();
  }, [
    key,
    companyId,
    query,
    sort,
    customer,
    status,
    page,
    pageSize,
    date_field,
    date_from,
    date_to,
    amount_min,
    amount_max,
    onExpired,
  ]);
  const change = (patch: Record<string, string>) =>
    setParams({
      ...Object.fromEntries(params),
      q: query,
      status,
      sort,
      customer,
      customerName: params.get("customerName") ?? "",
      page: "1",
      ...patch,
    });
  const loaded = result?.key === key;
  const data = loaded ? result.value : null;
  const editable = company?.id === companyId && company.role !== "viewer";
  return (
    <div className="space-y-group">
      <div className="flex flex-wrap items-center justify-between gap-field">
        <div>
          <Heading level={1} size="page">
            {t("navInvoices")}
          </Heading>
        </div>
      </div>
      <div>
        <Button
          size="sm"
          variant="secondary"
          aria-expanded={filtersOpen}
          aria-controls="invoice-filters"
          onClick={() => setFiltersOpen((v) => !v)}
        >
          <Icon name="settings" />
          {t("draftFilters")}
          {(customer || rangeActive) && (
            <Badge>{Number(!!customer) + Number(rangeActive)}</Badge>
          )}
        </Button>
        {filtersOpen && (
          <InvoiceListFilters
            key={
              companyId +
              [date_field, date_from, date_to, amount_min, amount_max].join("|")
            }
            company={companyId}
            params={params}
            onExpired={onExpired}
            onChange={change}
          />
        )}
      </div>
      <div className="invoice-sheet register-sheet overflow-hidden">
        <div className="invoice-list-tabs">
          <FilterTabs
            label={t("navInvoices")}
            value={status}
            onChange={(status) => change({ status })}
            options={[
              { value: "all", label: t("invoiceAll") },

              { value: "paid", label: t("paymentPaid") },
              { value: "outstanding", label: t("paymentOutstanding") },
              { value: "overdue", label: t("paymentOverdue") },
              { value: "draft", label: t("draftTab") },
            ]}
          />
        </div>
        <div className="p-field space-y-field">
          <div className="flex flex-wrap items-end justify-between gap-field">
            <form
              className="flex min-w-0 basis-full items-end gap-inline sm:basis-auto"
              onSubmit={(e) => {
                e.preventDefault();
                change({ q: input });
              }}
            >
              <Input
                className="w-full sm:w-72"
                aria-label={t("draftSearch")}
                placeholder={t("invoiceSearchPlaceholder")}
                value={input}
                maxLength={200}
                onChange={(e) => setInput(e.target.value)}
              />
              <Button
                variant="ghost"
                type="submit"
                aria-label={t("contactSearchAction")}
                title={t("contactSearchAction")}
              >
                <Icon name="search" />
              </Button>
            </form>
            {editable && (
              <Link
                className="ui-button ui-button-secondary ui-button-sm"
                to="new"
              >
                <Icon name="plus" />
                {t("draftNew")}
              </Link>
            )}
          </div>
          {(query || customer || rangeActive) && (
            <div className="flex items-center gap-inline">
              {query && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => change({ q: "" })}
                >
                  <Icon name="close" />
                  {t("draftSearchChip", { value: query })}
                </Button>
              )}
              {customer && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => change({ customer: "", customerName: "" })}
                >
                  <Icon name="close" />
                  {t("draftCustomerChip", {
                    value: params.get("customerName") || customer,
                  })}
                </Button>
              )}
              {rangeActive && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() =>
                    change({
                      date_from: "",
                      date_to: "",
                      amount_min: "",
                      amount_max: "",
                    })
                  }
                >
                  <Icon name="close" />
                  {t("invoiceFilterRangeChip")}
                </Button>
              )}
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  change({
                    q: "",
                    customer: "",
                    customerName: "",
                    date_from: "",
                    date_to: "",
                    amount_min: "",
                    amount_max: "",
                  })
                }
              >
                {t("draftClearAll")}
              </Button>
            </div>
          )}
        </div>
        <div className="border-t border-border">
          {error ? (
            <Alert tone="danger" role="alert">
              {t(error)}
              <Button
                variant="secondary"
                onClick={() => setAttempt((v) => v + 1)}
              >
                {t("retry")}
              </Button>
            </Alert>
          ) : !data ? (
            <p className="p-section text-sm text-muted" role="status">
              {t("draftLoading")}
            </p>
          ) : (
            <>
              {!data.items.length ? (
                <div className="p-section text-center">
                  <Heading level={2} size="card">
                    {t(query || customer ? "contactNoResults" : "draftEmpty")}
                  </Heading>
                  <p className="mt-control text-sm text-muted">
                    {t("draftEmptyHint")}
                  </p>
                </div>
              ) : (
                <div
                  className="overflow-x-auto"
                  role="region"
                  tabIndex={0}
                  aria-label={t("navInvoices")}
                >
                  <Table className="min-w-invoice-register invoice-register-table table-fixed">
                    <colgroup>
                      {[22, 6, 15, 15, 10, 10, 13, 9].map((width, i) => (
                        <col key={i} style={{ width: `${width}%` }} />
                      ))}
                    </colgroup>
                    <TableHeader>
                      <TableRow>
                        {(
                          [
                            ["draftDocument", "number"],
                            ["invoiceColumnStatus", ""],
                            ["invoiceDescription", ""],
                            ["draftCustomer", "customer"],
                            ["invoiceColumnIssued", "issued"],
                            ["invoiceColumnDue", "due"],
                            ["invoiceColumnAmount", "amount"],
                            ["invoiceListActions", ""],
                          ] as const
                        ).map(([label, field]) => (
                          <TableHead
                            key={label}
                            className={field === "amount" ? "text-right" : ""}
                            aria-sort={
                              field && sort.startsWith(field + "_")
                                ? sort.endsWith("asc")
                                  ? "ascending"
                                  : "descending"
                                : undefined
                            }
                          >
                            {field ? (
                              <button
                                className="invoice-sort"
                                onClick={() =>
                                  change({
                                    sort:
                                      field +
                                      (sort === field + "_asc"
                                        ? "_desc"
                                        : "_asc"),
                                  })
                                }
                              >
                                {t(label)}
                                {sort.startsWith(field + "_") && (
                                  <span aria-hidden="true">
                                    {sort.endsWith("asc") ? " ↑" : " ↓"}
                                  </span>
                                )}
                              </button>
                            ) : (
                              t(label)
                            )}
                          </TableHead>
                        ))}
                      </TableRow>
                    </TableHeader>
                    <tbody>
                      {data.items.map((d) => (
                        <TableRow key={d.id}>
                          <TableCell>
                            <div className="flex items-center gap-inline">
                              <Link
                                className="block min-w-0 truncate font-semibold hover:underline underline-offset-4"
                                to={d.id}
                                title={
                                  d.number ||
                                  d.data.calculation.lines[0]?.description.trim() ||
                                  t("draftUntitled")
                                }
                              >
                                {d.number ||
                                  d.data.calculation.lines[0]?.description.trim() ||
                                  t("draftUntitled")}
                              </Link>
                              {!d.number && (
                                <Badge>{t("draftListStatus")}</Badge>
                              )}
                            </div>
                          </TableCell>
                          <TableCell>
                            {d.settlement ? (
                              <PaymentStatus value={d.settlement} compact />
                            ) : d.number ? (
                              <Badge>{t("invoiceIssuedStatus")}</Badge>
                            ) : (
                              <span className="text-muted">—</span>
                            )}
                          </TableCell>
                          <TableCell className="max-w-xs">
                            <p
                              className="truncate"
                              title={d.data.calculation.lines[0]?.description}
                            >
                              {d.data.calculation.lines[0]?.description || "—"}
                            </p>
                          </TableCell>
                          <TableCell className="truncate">
                            {d.data.customer?.name || t("notProvided")}
                          </TableCell>
                          <TableCell className="whitespace-nowrap tabular-nums text-muted">
                            {d.data.issued_on
                              ? formatCalendarDate(d.data.issued_on, language)
                              : "—"}
                          </TableCell>
                          <TableCell className="whitespace-nowrap tabular-nums text-muted">
                            {d.data.due_on
                              ? formatCalendarDate(d.data.due_on, language)
                              : "—"}
                          </TableCell>
                          <TableCell className="whitespace-nowrap text-right font-semibold tabular-nums">
                            {d.totals
                              ? moneyText(d.totals.payable, language)
                              : t("draftIncomplete")}
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center justify-end gap-inline">
                              {d.number &&
                                d.data.calculation.calculation_version !==
                                  "import-v1" && (
                                  <a
                                    className="ui-button ui-button-ghost ui-button-sm ui-icon-button"
                                    href={`${apiUrl}/api/companies/${encodeURIComponent(companyId)}/invoices/${encodeURIComponent(d.id)}/pdf`}
                                    target="_blank"
                                    rel="noreferrer"
                                    aria-label={t("invoiceRowPdf", {
                                      number: d.number,
                                    })}
                                    title={t("invoiceRowPdf", {
                                      number: d.number,
                                    })}
                                  >
                                    <Icon name="eye" />
                                  </a>
                                )}
                              {editable &&
                                d.data.calculation.calculation_version !==
                                  "import-v1" && (
                                  <Link
                                    className="ui-button ui-button-ghost ui-button-sm ui-icon-button"
                                    to={`new?copy=${encodeURIComponent(d.id)}`}
                                    aria-label={t("invoiceRowCopy", {
                                      name:
                                        d.number ||
                                        d.data.calculation.lines[0]
                                          ?.description ||
                                        t("draftUntitled"),
                                    })}
                                    title={t("invoiceRowCopy", {
                                      name:
                                        d.number ||
                                        d.data.calculation.lines[0]
                                          ?.description ||
                                        t("draftUntitled"),
                                    })}
                                  >
                                    <Icon name="copy" />
                                  </Link>
                                )}
                            </div>
                          </TableCell>
                        </TableRow>
                      ))}
                    </tbody>
                  </Table>
                </div>
              )}
              <RegisterPagination
                page={page}
                pageSize={data.page_size}
                total={data.total}
                onPage={(page) => change({ page: String(page) })}
                onPageSize={(page_size) => change({ page_size })}
              />
            </>
          )}
        </div>
      </div>
    </div>
  );
}
