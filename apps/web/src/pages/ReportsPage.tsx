import { DataChart } from "@open-faktura/ui/charts";
import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import {
  Alert,
  Button,
  DatePicker,
  Select,
  Heading,
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  periodReport,
  SalesFailure,
  type PeriodReport,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
const kinds: Record<string, [string, AppKey]> = {
  invoice: ["invoices", "invoiceIssuedStatus"],
  credit_note: ["credit-notes", "navCreditNotes"],
  retail: ["retail", "retailTitle"],
  received: ["received-invoices", "receivedTitle"],
  purchase_receipt: ["receipts", "receiptsTitle"],
};
export function ReportsPage() {
  const { companyId = "" } = useParams(),
    { t, language } = useAppTranslation(),
    { onExpired } = useWorkspace();
  const today = new Date().toLocaleDateString("sv-SE");
  const [from, setFrom] = useState(today.slice(0, 4) + "-01-01"),
    [to, setTo] = useState(today),
    [page, setPage] = useState(1),
    [valueMode, setValueMode] = useState("gross"),
    [cumulative, setCumulative] = useState("false"),
    [compare, setCompare] = useState("false"),
    [attempt, setAttempt] = useState(0),
    [value, setValue] = useState<PeriodReport | null>(null),
    [error, setError] = useState(false);
  useEffect(() => {
    const c = new AbortController();
    setValue(null);
    setError(false);
    periodReport(
      apiUrl,
      companyId,
      {
        from,
        to,
        page: String(page),
        value_mode: valueMode,
        cumulative,
        compare_previous: compare,
      },
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
  }, [
    companyId,
    from,
    to,
    page,
    valueMode,
    cumulative,
    compare,
    attempt,
    onExpired,
  ]);
  return (
    <div className="space-y-group">
      <Heading level={1} size="page">
        {t("navReports")}
      </Heading>
      <p className="text-muted">{t("reportScope")}</p>
      <div className="workspace-panel p-field flex flex-wrap gap-field">
        <Select
          label={t("reportValueMode")}
          value={valueMode}
          onChange={(v) => {
            setValueMode(v);
            setPage(1);
          }}
          options={[
            { value: "gross", label: t("reportGross") },
            { value: "net", label: t("reportNet") },
          ]}
        />
        <Select
          label={t("reportDisplay")}
          value={cumulative}
          onChange={setCumulative}
          options={[
            { value: "false", label: t("reportByMonth") },
            { value: "true", label: t("reportCumulative") },
          ]}
        />
        <Select
          label={t("reportCompare")}
          value={compare}
          onChange={setCompare}
          options={[
            { value: "false", label: t("reportNoComparison") },
            { value: "true", label: t("reportPreviousYear") },
          ]}
        />
        <DatePicker
          label={t("reportFrom")}
          value={from}
          onChange={(v) => {
            setFrom(v);
            setPage(1);
          }}
        />

        <DatePicker
          label={t("reportTo")}
          value={to}
          onChange={(v) => {
            setTo(v);
            setPage(1);
          }}
        />
      </div>
      {error && (
        <Alert tone="danger" role="alert">
          {t("reportError")}
          <Button variant="secondary" onClick={() => setAttempt((v) => v + 1)}>
            {t("retry")}
          </Button>
        </Alert>
      )}
      {!value && !error && <p role="status">{t("draftLoading")}</p>}
      {value && (
        <>
          <div className="grid sm:grid-cols-2 gap-field">
            {(
              [
                ["reportSales", value.sales],
                ["reportPurchases", value.purchases],
              ] as const
            ).map(([label, amount]) => (
              <section key={label} className="workspace-panel p-field">
                <Heading level={2} size="card">
                  {t(label)}
                </Heading>
                <p className="text-2xl font-semibold tabular-nums">
                  {moneyText(amount, language)}
                </p>
              </section>
            ))}
          </div>
          {value.comparison ? (
            <>
              <section className="workspace-panel p-field space-y-field">
                <Heading level={2} size="card">
                  {t("reportPreviousYear")}
                </Heading>
                <p>
                  {formatCalendarDate(value.comparison.from, language)} –{" "}
                  {formatCalendarDate(value.comparison.to, language)}
                </p>
                <div className="grid sm:grid-cols-2 gap-field">
                  {(["sales", "purchases"] as const).map((side) => (
                    <div key={side}>
                      <p>
                        {t(
                          side === "sales" ? "reportSales" : "reportPurchases",
                        )}
                        : {moneyText(value.comparison![side], language)}
                      </p>
                      <p>
                        {t("reportDifference")}:{" "}
                        {moneyText(
                          value.comparison![
                            side === "sales"
                              ? "sales_difference"
                              : "purchases_difference"
                          ],
                          language,
                        )}
                      </p>
                    </div>
                  ))}
                </div>
                <Button
                  variant="secondary"
                  onClick={() => {
                    setFrom(value.comparison!.from);
                    setTo(value.comparison!.to);
                    setCompare("false");
                    setPage(1);
                  }}
                >
                  {t("reportPreviousSources")}
                </Button>
              </section>
              {(["sales", "purchases"] as const).map((side) => (
                <DataChart
                  key={side}
                  title={t(
                    side === "sales" ? "reportSales" : "reportPurchases",
                  )}
                  description={t(
                    cumulative === "true"
                      ? "reportCumulativeHint"
                      : "reportMonthlyHint",
                  )}
                  variant={cumulative === "true" ? "line" : "bar"}
                  series={[
                    { key: "current", label: t("reportCurrentPeriod") },
                    { key: "previous", label: t("reportPreviousYear") },
                  ]}
                  data={value.months.map((m, index) => ({
                    label: m.month,
                    values: {
                      current: Number(m[side]),
                      previous: Number(
                        value.comparison!.months[index]?.[side] ?? "0",
                      ),
                    },
                    formattedValues: {
                      current: moneyText(m[side], language),
                      previous: moneyText(
                        value.comparison!.months[index]?.[side] ?? "0",
                        language,
                      ),
                    },
                  }))}
                />
              ))}
            </>
          ) : (
            <>
              <DataChart
                title={t(
                  cumulative === "true" ? "reportCumulative" : "reportMonthly",
                )}
                description={t(
                  cumulative === "true"
                    ? "reportCumulativeHint"
                    : "reportMonthlyHint",
                )}
                variant={cumulative === "true" ? "line" : "bar"}
                series={[
                  { key: "sales", label: t("reportSales") },
                  { key: "purchases", label: t("reportPurchases") },
                ]}
                data={value.months.map((m) => ({
                  label: m.month,
                  values: {
                    sales: Number(m.sales),
                    purchases: Number(m.purchases),
                  },
                  formattedValues: {
                    sales: moneyText(m.sales, language),
                    purchases: moneyText(m.purchases, language),
                  },
                }))}
              />
            </>
          )}
          <section className="workspace-panel">
            <Heading level={2} size="card" className="p-field">
              {t("reportSources")}
            </Heading>
            <div
              className="overflow-x-auto"
              role="region"
              aria-label={t("reportSources")}
              tabIndex={0}
            >
              <Table className="whitespace-nowrap">
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("reportDocument")}</TableHead>
                    <TableHead>{t("reportDate")}</TableHead>
                    <TableHead>{t("reportSide")}</TableHead>
                    <TableHead className="text-right">
                      {t("reportAmount")}
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <tbody>
                  {value.items.map((d) => (
                    <TableRow key={d.kind + d.id}>
                      <TableCell>
                        <Link
                          className="underline underline-offset-4"
                          to={`/companies/${companyId}/${kinds[d.kind]?.[0] ?? "invoices"}/${d.id}`}
                        >
                          {d.number}
                        </Link>
                        <span className="block text-xs text-muted">
                          {kinds[d.kind] && t(kinds[d.kind][1])}
                        </span>
                      </TableCell>
                      <TableCell>
                        {formatCalendarDate(d.reported_on, language)}
                      </TableCell>
                      <TableCell>
                        {t(
                          d.side === "sales"
                            ? "reportSales"
                            : "reportPurchases",
                        )}
                      </TableCell>
                      <TableCell className="tabular-nums text-right">
                        {moneyText(d.amount, language)}
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!value.total && (
              <p className="p-field text-muted">{t("reportEmpty")}</p>
            )}
            <div className="flex flex-wrap items-center justify-between gap-field p-field">
              <span>{t("reportCount", { count: value.total })}</span>
              <div className="flex gap-control">
                <Button
                  variant="secondary"
                  disabled={page <= 1}
                  onClick={() => setPage((p) => p - 1)}
                >
                  {t("reportPrevious")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={page * 25 >= value.total}
                  onClick={() => setPage((p) => p + 1)}
                >
                  {t("reportNext")}
                </Button>
              </div>
            </div>
          </section>
        </>
      )}
    </div>
  );
}
