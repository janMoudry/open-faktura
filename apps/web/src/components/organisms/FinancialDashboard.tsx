import { useEffect, useState } from "react";
import { Link } from "react-router";
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
} from "@open-faktura/ui";
import {
  dashboardReport,
  SalesFailure,
  type Dashboard,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function FinancialDashboard({
  company,
  today,
  refresh,
  onExpired,
}: {
  company: string;
  today: string;
  refresh: number;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const current = today.slice(0, 4);
  const [year, setYear] = useState(current),
    [scope, setScope] = useState("false"),
    [net, setNet] = useState("false"),
    [value, setValue] = useState<Dashboard | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    setValue(null);
    setError(false);
    dashboardReport(
      apiUrl,
      company,
      { year, all_years: scope },
      controller.signal,
    )
      .then((v) => {
        if (!controller.signal.aborted) setValue(v);
      })
      .catch((e) => {
        if (!controller.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => controller.abort();
  }, [company, year, scope, refresh, attempt, onExpired]);
  return (
    <section className="space-y-group" aria-label={t("dashboardTitle")}>
      <Heading level={2} size="section">
        {t("dashboardTitle")}
      </Heading>
      <div className="workspace-panel p-field flex flex-wrap gap-field">
        <Select
          label={t("dashboardYear")}
          value={year}
          onChange={setYear}
          options={[
            { value: current, label: current },
            {
              value: String(Number(current) - 1),
              label: String(Number(current) - 1),
            },
          ]}
        />
        <Select
          label={t("reportValueMode")}
          value={net}
          onChange={setNet}
          options={[
            { value: "false", label: t("reportGross") },
            { value: "true", label: t("reportNet") },
          ]}
        />
        <Select
          label={t("dashboardAgingScope")}
          value={scope}
          onChange={setScope}
          options={[
            { value: "false", label: t("dashboardSelectedYear") },
            { value: "true", label: t("dashboardAllYears") },
          ]}
        />
      </div>
      <p className="text-sm text-muted">
        {t("dashboardScope")} {t("exchangeReportHint")}
      </p>
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
          <div className="grid gap-field md:grid-cols-2">
            {(["sales", "purchases"] as const).map((side) => (
              <section
                key={side}
                className="workspace-panel p-field space-y-field"
              >
                <Heading level={3} size="card">
                  {t(side === "sales" ? "reportSales" : "reportPurchases")}
                </Heading>
                <p className="text-2xl font-semibold tabular-nums">
                  {moneyText(
                    value[side][net === "true" ? "net" : "gross"],
                    language,
                  )}
                </p>
                <dl className="grid gap-field sm:grid-cols-2">
                  {(["settled", "outstanding"] as const).map((k) => (
                    <div key={k}>
                      <dt className="text-sm text-muted">
                        {t(
                          k === "settled"
                            ? "dashboardSettled"
                            : "dashboardRemaining",
                        )}
                      </dt>
                      <dd className="font-semibold tabular-nums">
                        {moneyText(value[side][k], language)}
                      </dd>
                    </div>
                  ))}
                </dl>
              </section>
            ))}
          </div>
          <p className="text-sm text-muted">{t("dashboardSettlementHint")}</p>
          <div className="grid gap-field lg:grid-cols-2">
            {(["sales", "purchases"] as const).map((side) => (
              <section key={side} className="workspace-panel overflow-hidden">
                <Heading level={3} size="card" className="p-field">
                  {t(
                    side === "sales"
                      ? "dashboardReceivables"
                      : "dashboardPayables",
                  )}
                </Heading>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("dashboardDelay")}</TableHead>
                      <TableHead className="text-right">
                        {t("reportAmount")}
                      </TableHead>
                      <TableHead className="text-right">
                        {t("dashboardCount")}
                      </TableHead>
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {value.aging
                      .filter((r) => r.side === side)
                      .map((r) => (
                        <TableRow key={r.bucket}>
                          <TableCell>
                            {t(("dashboard_" + r.bucket) as AppKey)}
                          </TableCell>
                          <TableCell className="text-right tabular-nums whitespace-nowrap">
                            {moneyText(r.amount, language)}
                          </TableCell>
                          <TableCell className="text-right">
                            {r.count}
                          </TableCell>
                        </TableRow>
                      ))}
                  </tbody>
                </Table>
              </section>
            ))}
          </div>
          <Heading level={3} size="card">
            {t("dashboardPartners")}
          </Heading>
          <p className="text-sm text-muted">{t("dashboardPartnersHint")}</p>
          <div className="grid gap-field lg:grid-cols-2">
            {(["customers", "debtors"] as const).map((kind) => (
              <section key={kind} className="workspace-panel overflow-hidden">
                <Heading level={4} size="card" className="p-field">
                  {t(
                    kind === "customers"
                      ? "dashboardCustomers"
                      : "dashboardDebtors",
                  )}
                </Heading>
                {value[kind].length ? (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t("dashboardPartner")}</TableHead>
                        <TableHead className="text-right">
                          {t("reportAmount")}
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <tbody>
                      {value[kind].map((r, i) => (
                        <TableRow key={i}>
                          <TableCell>
                            {r.contact_id ? (
                              <Link
                                className="underline underline-offset-4"
                                to={`/companies/${company}/contacts/${r.contact_id}`}
                              >
                                {r.name || t("dashboardUnknown")}
                              </Link>
                            ) : (
                              r.name || t("dashboardUnknown")
                            )}
                          </TableCell>
                          <TableCell className="text-right whitespace-nowrap tabular-nums">
                            {moneyText(r.amount, language)}
                          </TableCell>
                        </TableRow>
                      ))}
                    </tbody>
                  </Table>
                ) : (
                  <p className="p-field text-muted">{t("reportEmpty")}</p>
                )}
              </section>
            ))}
          </div>
        </>
      )}
    </section>
  );
}
