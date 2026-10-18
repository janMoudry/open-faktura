import { TaxDashboard } from "../components/organisms/TaxDashboard";
import { FinancialDashboard } from "../components/organisms/FinancialDashboard";
import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import {
  Alert,
  Badge,
  Button,
  Heading,
  Icon,
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import { DataChart } from "@open-faktura/ui/charts";
import {
  listDrafts,
  periodReport,
  SalesFailure,
  InvoiceCalculationFailure,
  type DraftPage,
  type PeriodReport,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useInvoiceOverview } from "../hooks/useInvoiceOverview";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
import { PaymentStatus } from "../components/molecules/PaymentStatus";

export function OverviewPage() {
  const { companyId = "" } = useParams();
  const { t, language } = useAppTranslation();
  const { company, onExpired } = useWorkspace();
  const overview = useInvoiceOverview(companyId, onExpired);
  const value = overview.value;
  const [report, setReport] = useState<{
    key: string;
    value: PeriodReport;
  } | null>(null);
  const [recent, setRecent] = useState<{
    company: string;
    value: DraftPage;
  } | null>(null);
  const [reportError, setReportError] = useState(false);
  const [recentError, setRecentError] = useState(false);
  const today = value?.today;
  const reportKey = `${companyId}:${today ?? ""}`;
  useEffect(() => {
    if (!today) return;
    const controller = new AbortController();
    setReportError(false);
    setRecentError(false);
    const failed = (error: unknown) => {
      if (
        (error instanceof SalesFailure ||
          error instanceof InvoiceCalculationFailure) &&
        error.status === 401
      )
        onExpired();
    };
    periodReport(
      apiUrl,
      companyId,
      {
        from: today.slice(0, 4) + "-01-01",
        to: today,
        page: "1",
        value_mode: "gross",
        cumulative: "false",
        compare_previous: "false",
      },
      controller.signal,
    )
      .then((data) => {
        if (!controller.signal.aborted)
          setReport({ key: reportKey, value: data });
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          failed(error);
          setReportError(true);
        }
      });
    listDrafts(
      apiUrl,
      companyId,
      {
        query: "",
        page: 1,
        page_size: 10,
        sort: "updated_desc",
        status: "all",
      },
      controller.signal,
    )
      .then((data) => {
        if (!controller.signal.aborted)
          setRecent({ company: companyId, value: data });
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          failed(error);
          setRecentError(true);
        }
      });
    return () => controller.abort();
  }, [companyId, today, reportKey, onExpired, value]);
  const currentReport =
    report?.key === reportKey && !reportError ? report.value : null;
  const invoices =
    recent?.company === companyId && !recentError
      ? recent.value.items.slice(0, 4)
      : null;
  return (
    <div className="space-y-group">
      <div className="page-heading">
        <div className="page-heading-copy">
          <Heading level={1} size="page">
            {t("navOverview")}
          </Heading>
          <p className="text-sm text-muted">{t("overviewWelcome")}</p>
        </div>
        <div className="page-heading-actions">
          {today && (
            <span className="inline-flex items-center gap-inline text-sm text-muted">
              <Icon name="calendar" />
              {formatCalendarDate(today, language)}
            </span>
          )}
          {company?.id === companyId && company.role !== "viewer" && (
            <Link
              className="ui-button ui-button-primary ui-button-md"
              to={`../invoices/new`}
              relative="path"
            >
              <Icon name="plus" />
              {t("draftNew")}
            </Link>
          )}
        </div>
      </div>
      {overview.failed && (
        <Alert tone="danger" role="alert">
          {t("requestFailed")}
          <Button variant="secondary" onClick={overview.retry}>
            {t("retry")}
          </Button>
        </Alert>
      )}
      {!value && !overview.failed && <p role="status">{t("draftLoading")}</p>}
      {value && (
        <>
          <p className="text-sm text-muted">
            {t("overviewAsOf", {
              date: formatCalendarDate(value.today, language),
            })}
          </p>
          <FinancialDashboard
            key={companyId}
            company={companyId}
            today={value.today}
            refresh={overview.refresh}
            onExpired={onExpired}
          />
          <TaxDashboard
            key={"tax-" + companyId}
            company={companyId}
            today={value.today}
            refresh={overview.refresh}
            onExpired={onExpired}
          />
          <div className="overview-metrics">
            {(
              [
                ["overviewSales", currentReport?.sales, "invoice"],
                ["overviewPurchases", currentReport?.purchases, "expenses"],
                ["paymentOutstanding", value.outstanding, "contacts"],
                ["paymentOverdue", value.overdue, "warning"],
              ] as const
            ).map(([label, amount, icon], index) => (
              <section className="overview-metric" key={label}>
                <Heading level={2} size="label" className="metric-label">
                  <Icon name={icon} />
                  {t(label)}
                </Heading>
                <p className="metric-value">
                  {amount !== undefined
                    ? moneyText(amount, language)
                    : reportError
                      ? t("notProvided")
                      : "…"}
                </p>
                <p className="metric-hint">
                  {index < 2 ? t("overviewThisYear") : t("overviewAllHistory")}
                </p>
              </section>
            ))}
          </div>
          <div className="overview-grid">
            <DataChart
              title={t("overviewYearTrend")}
              description={t("overviewTurnoverScope")}
              variant="bar"
              onRetry={overview.retry}
              loading={!currentReport && !reportError}
              error={reportError ? t("overviewReportUnavailable") : undefined}
              data={(currentReport?.months ?? []).map((month) => ({
                label: month.month,
                values: {
                  sales: Number(month.sales),
                  purchases: Number(month.purchases),
                },
                formattedValues: {
                  sales: moneyText(month.sales, language),
                  purchases: moneyText(month.purchases, language),
                },
              }))}
              series={[
                { key: "sales", label: t("reportSales") },
                { key: "purchases", label: t("reportPurchases") },
              ]}
              formatValue={(amount) => moneyText(String(amount), language)}
              formatAxis={(amount) =>
                new Intl.NumberFormat(language, {
                  notation: "compact",
                  maximumFractionDigits: 1,
                }).format(amount)
              }
            />
            <section className="workspace-panel">
              <div className="overview-section-heading">
                <Heading level={2} size="card">
                  {t("overviewAttention")}
                </Heading>
              </div>
              {value.overdue_count > 0 ? (
                <Link
                  className="overview-attention-link"
                  to="../invoices?status=overdue"
                  relative="path"
                >
                  <Icon name="warning" className="h-5 w-5 text-danger" />
                  <div>
                    <p className="text-sm font-semibold">
                      {t("paymentOverdue")}
                    </p>
                    <p className="text-sm text-danger">
                      {moneyText(value.overdue, language)}
                    </p>
                    <p className="text-xs text-muted">
                      {t("overviewInvoiceCount", {
                        count: value.overdue_count,
                      })}
                    </p>
                  </div>
                  <Icon name="chevronRight" />
                </Link>
              ) : (
                <p className="border-t border-border p-group text-sm text-success">
                  {t("overviewNoAttention")}
                </p>
              )}
              <Link
                className="overview-attention-link"
                to="../invoices?status=draft"
                relative="path"
              >
                <Icon name="invoice" className="h-5 w-5" />
                <div>
                  <p className="text-sm font-semibold">{t("draftTab")}</p>
                  <p className="text-sm text-muted">{value.draft_count}</p>
                  <p className="text-xs text-muted">
                    {t("overviewDraftsHint")}
                  </p>
                </div>
                <Icon name="chevronRight" />
              </Link>
              <Link
                className="overview-attention-link"
                to="../reports"
                relative="path"
              >
                <Icon name="overview" className="h-5 w-5" />
                <div>
                  <p className="text-sm font-semibold">
                    {t("overviewReportsLink")}
                  </p>
                  <p className="text-xs text-muted">
                    {t("overviewTurnoverScope")}
                  </p>
                </div>
                <Icon name="chevronRight" />
              </Link>
            </section>
          </div>
          <div className="overview-grid">
            <section className="workspace-panel">
              <div className="overview-section-heading">
                <Heading level={2} size="card">
                  {t("overviewRecent")}
                </Heading>
                <Link
                  className="inline-flex items-center gap-inline text-sm font-medium text-primary hover:underline"
                  to="../invoices"
                  relative="path"
                >
                  {t("overviewAllInvoices")}
                  <Icon name="arrow" />
                </Link>
              </div>
              {recentError ? (
                <p role="status" className="p-group text-sm text-muted">
                  {t("overviewInvoicesUnavailable")}
                </p>
              ) : !invoices ? (
                <p role="status" className="p-group text-sm text-muted">
                  {t("draftLoading")}
                </p>
              ) : !invoices.length ? (
                <p className="p-group text-sm text-muted">{t("draftEmpty")}</p>
              ) : (
                <div
                  className="overflow-x-auto"
                  role="region"
                  tabIndex={0}
                  aria-label={t("overviewRecent")}
                >
                  <Table className="overview-recent-table">
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t("draftDocument")}</TableHead>
                        <TableHead>{t("draftCustomer")}</TableHead>
                        <TableHead>{t("invoiceColumnStatus")}</TableHead>
                        <TableHead className="text-right">
                          {t("invoiceColumnAmount")}
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <tbody>
                      {invoices.map((invoice) => (
                        <TableRow key={invoice.id}>
                          <TableCell>
                            <Link
                              className="font-semibold hover:underline"
                              to={`../invoices/${invoice.id}`}
                              relative="path"
                            >
                              {invoice.number ||
                                invoice.data.calculation.lines[0]
                                  ?.description ||
                                t("draftUntitled")}
                            </Link>
                            <p className="mt-1 text-xs text-muted">
                              {invoice.data.issued_on
                                ? formatCalendarDate(
                                    invoice.data.issued_on,
                                    language,
                                  )
                                : "—"}
                            </p>
                          </TableCell>
                          <TableCell>
                            {invoice.data.customer?.name || t("notProvided")}
                          </TableCell>
                          <TableCell>
                            {invoice.settlement ? (
                              <PaymentStatus value={invoice.settlement} />
                            ) : (
                              <Badge>
                                {t(
                                  invoice.number
                                    ? "invoiceIssuedStatus"
                                    : "draftListStatus",
                                )}
                              </Badge>
                            )}
                          </TableCell>
                          <TableCell className="whitespace-nowrap text-right font-semibold tabular-nums">
                            {invoice.totals
                              ? moneyText(invoice.totals.payable, language)
                              : t("draftIncomplete")}
                          </TableCell>
                        </TableRow>
                      ))}
                    </tbody>
                  </Table>
                </div>
              )}
            </section>
            <section className="workspace-panel p-group space-y-field">
              <Heading level={2} size="card">
                {t("overviewDocuments")}
              </Heading>
              <div className="overview-documents">
                {(
                  [
                    ["invoiceIssuedStatus", value.issued_count, "issued"],
                    ["draftTab", value.draft_count, "draft"],
                    [
                      "paymentOutstanding",
                      value.outstanding_count,
                      "outstanding",
                    ],
                    ["paymentOverdue", value.overdue_count, "overdue"],
                  ] as const
                ).map(([label, count, status]) => (
                  <Link
                    key={label}
                    to={`../invoices?status=${status}`}
                    relative="path"
                  >
                    <span className="text-xs text-muted">{t(label)}</span>
                    <span className="text-xl font-semibold tabular-nums">
                      {count}
                    </span>
                  </Link>
                ))}
              </div>
              <p className="text-xs leading-relaxed text-muted">
                {t("overviewScope")}
              </p>
            </section>
          </div>
        </>
      )}
    </div>
  );
}
