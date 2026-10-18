import { TaxpayerFilingProfile } from "./TaxpayerFilingProfile";
import {
  TaxEvidenceClassification,
  TaxClassificationSummary,
} from "./TaxEvidenceClassification";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { Link } from "react-router";
import { useEffect, useState } from "react";
import {
  Alert,
  Button,
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
  taxReport,
  exportTaxEvidence,
  SalesFailure,
  type TaxReport,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function TaxDashboard({
  company,
  compact = false,
  today,
  refresh,
  onExpired,
}: {
  company: string;
  compact?: boolean;
  today: string;
  refresh: number;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const workspace = useWorkspace();
  const canClassify =
    workspace.company?.id === company && workspace.company.role !== "viewer";
  const [classificationRefresh, setClassificationRefresh] = useState(0);
  const [exporting, setExporting] = useState(false),
    [exportFailed, setExportFailed] = useState(false);
  async function download(format: "csv" | "xlsx") {
    setExporting(true);
    setExportFailed(false);
    try {
      const file = await exportTaxEvidence(apiUrl, company, {
        search: {
          year: Number(year),
          quarterly: quarterly === "true",
          turnover_sources: source === "true",
        },
        format,
        language,
      });
      const url = URL.createObjectURL(file),
        a = document.createElement("a");
      a.href = url;
      a.download = `${source === "true" ? "turnover-evidence" : "vat-evidence"}-${year}.${format}`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) {
      setExportFailed(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setExporting(false);
    }
  }
  const current = today.slice(0, 4);
  const [year, setYear] = useState(current),
    [quarterly, setQuarterly] = useState("false"),
    [source, setSource] = useState("true"),
    [page, setPage] = useState(1),
    [value, setValue] = useState<TaxReport | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const c = new AbortController();
    setValue(null);
    setError(false);
    taxReport(
      apiUrl,
      company,
      { year, quarterly, turnover_sources: source, page: String(page) },
      c.signal,
    )
      .then((v) => {
        if (!c.signal.aborted) setValue(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [company, year, quarterly, source, page, refresh, attempt, onExpired]);
  const amount = (v: string) => moneyText(v, language);
  return (
    <section className="space-y-group" aria-label={t("taxDashboardTitle")}>
      <Heading level={2} size="section">
        {t("taxDashboardTitle")}
      </Heading>
      <div className="workspace-panel p-field flex flex-wrap gap-field">
        <Select
          label={t("dashboardYear")}
          value={year}
          onChange={(v) => {
            setYear(v);
            setPage(1);
          }}
          options={[current, String(Number(current) - 1)].map((v) => ({
            value: v,
            label: v,
          }))}
        />
        <Select
          label={t("taxDashboardGrouping")}
          value={quarterly}
          onChange={setQuarterly}
          options={[
            { value: "false", label: t("reportMonthly") },
            { value: "true", label: t("reportQuarterly") },
          ]}
        />
      </div>
      <div className="flex flex-wrap gap-control">
        <Button
          variant="secondary"
          size="sm"
          loading={exporting}
          onClick={() => download("csv")}
        >
          {t("taxEvidenceCsv")}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={exporting}
          onClick={() => download("xlsx")}
        >
          {t("taxEvidenceXlsx")}
        </Button>
      </div>
      {exportFailed && (
        <Alert tone="danger" role="alert">
          {t("exportError")}
        </Alert>
      )}
      <p className="text-sm text-muted">
        {t("taxDashboardScope")} {t("exchangeReportHint")}
      </p>
      <TaxpayerFilingProfile
        key={company}
        company={company}
        editable={canClassify}
        onExpired={onExpired}
      />
      <TaxClassificationSummary
        company={company}
        year={Number(year)}
        refresh={refresh + classificationRefresh}
        onExpired={onExpired}
      />
      {error && (
        <Alert tone="danger" role="alert">
          {t("requestFailed")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((v) => v + 1)}>
            {t("retry")}
          </Button>
        </Alert>
      )}
      {!value && !error && <p role="status">{t("draftLoading")}</p>}
      {value && (
        <>
          <section className="workspace-panel p-field space-y-field">
            <Heading level={3} size="card">
              {t("taxDashboardTurnover")}
            </Heading>
            <p className="text-2xl font-semibold tabular-nums">
              {amount(value.turnover)}
            </p>
            <p className="text-sm text-muted">
              {t("taxDashboardTurnoverHint")}
            </p>
          </section>
          {value.tax_mode === "vat_payer" && !compact && (
            <section className="workspace-panel overflow-hidden">
              <Heading level={3} size="card" className="p-field">
                {t("taxDashboardVat")}
              </Heading>
              <div
                className="overflow-x-auto focus-visible:outline focus-visible:outline-2 focus-visible:outline-focus"
                tabIndex={0}
                role="region"
                aria-label={t("taxDashboardVat")}
              >
                <Table>
                  <TableHeader>
                    <TableRow>
                      {(
                        [
                          "taxDashboardPeriod",
                          "taxDashboardOutput",
                          "taxDashboardInput",
                          "taxDashboardDifference",
                        ] as const
                      ).map((k, i) => (
                        <TableHead
                          key={k}
                          className={
                            i
                              ? "text-right whitespace-nowrap"
                              : "whitespace-nowrap"
                          }
                        >
                          {t(k)}
                        </TableHead>
                      ))}
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {value.periods.map((r) => (
                      <TableRow key={r.period}>
                        <TableCell className="whitespace-nowrap">
                          {r.period}
                        </TableCell>
                        {[r.output, r.input, r.difference].map((v, i) => (
                          <TableCell
                            key={i}
                            className="text-right whitespace-nowrap tabular-nums"
                          >
                            {amount(v)}
                          </TableCell>
                        ))}
                      </TableRow>
                    ))}
                    <TableRow>
                      <TableCell className="whitespace-nowrap">
                        {t("taxDashboardTotal")}
                      </TableCell>
                      {[value.output, value.input, value.difference].map(
                        (v, i) => (
                          <TableCell
                            key={i}
                            className="text-right whitespace-nowrap tabular-nums font-semibold"
                          >
                            {amount(v)}
                          </TableCell>
                        ),
                      )}
                    </TableRow>
                  </tbody>
                </Table>
              </div>
            </section>
          )}
          {compact && value.tax_mode === "vat_payer" && (
            <dl className="grid gap-field sm:grid-cols-3">
              {(
                [
                  ["taxDashboardOutput", value.output],
                  ["taxDashboardInput", value.input],
                  ["taxDashboardDifference", value.difference],
                ] as const
              ).map(([label, value]) => (
                <div key={label}>
                  <dt>{t(label)}</dt>
                  <dd className="font-semibold tabular-nums">
                    {amount(value)}
                  </dd>
                </div>
              ))}
            </dl>
          )}
          {!compact && (
            <section className="workspace-panel overflow-hidden">
              <div className="p-field">
                <Select
                  label={t("taxDashboardSources")}
                  value={source}
                  onChange={(v) => {
                    setSource(v);
                    setPage(1);
                  }}
                  options={[
                    { value: "true", label: t("taxDashboardTurnover") },
                    { value: "false", label: t("taxDashboardVat") },
                  ]}
                />
              </div>
              {value.items.length ? (
                <div className="overflow-x-auto">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t("taxDashboardDocument")}</TableHead>
                        <TableHead>{t("taxDashboardDate")}</TableHead>
                        <TableHead>{t("taxDashboardSide")}</TableHead>
                        <TableHead>{t("taxClassificationTitle")}</TableHead>
                        <TableHead className="text-right">
                          {t("reportAmount")}
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <tbody>
                      {value.items.map((r) => (
                        <TableRow key={r.kind + ":" + r.id}>
                          <TableCell className="whitespace-nowrap">
                            <Link
                              className="underline underline-offset-4"
                              to={`/companies/${company}/${({ cash: "cash", invoice: "invoices", credit_note: "credit-notes", tax_receipt: "tax-receipts", retail: "retail", received: "received-invoices", purchase_receipt: "receipts" } as Record<string, string>)[r.kind]}/${r.id}`}
                            >
                              {r.number}
                            </Link>
                          </TableCell>
                          <TableCell className="whitespace-nowrap">
                            {formatCalendarDate(r.reported_on, language)}
                          </TableCell>
                          <TableCell className="whitespace-nowrap">
                            {t(
                              r.side === "sales"
                                ? "reportSales"
                                : "reportPurchases",
                            )}
                          </TableCell>
                          <TableCell>
                            <TaxEvidenceClassification
                              company={company}
                              kind={r.kind}
                              id={r.id}
                              editable={canClassify}
                              onChanged={() =>
                                setClassificationRefresh((v) => v + 1)
                              }
                              onExpired={onExpired}
                            />
                          </TableCell>
                          <TableCell className="text-right whitespace-nowrap tabular-nums">
                            {amount(r.amount)}
                          </TableCell>
                        </TableRow>
                      ))}
                    </tbody>
                  </Table>
                </div>
              ) : (
                <p className="p-field text-muted">{t("reportEmpty")}</p>
              )}
              <div className="p-field flex items-center justify-between gap-field">
                <span>
                  {t("taxDashboardDocuments", { count: value.total })}
                </span>
                <div className="flex gap-control">
                  <Button
                    variant="secondary"
                    disabled={page === 1}
                    onClick={() => setPage((v) => v - 1)}
                  >
                    {t("reportPrevious")}
                  </Button>
                  <Button
                    variant="secondary"
                    disabled={page * 25 >= value.total}
                    onClick={() => setPage((v) => v + 1)}
                  >
                    {t("reportNext")}
                  </Button>
                </div>
              </div>
            </section>
          )}
        </>
      )}
    </section>
  );
}
