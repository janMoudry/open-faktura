import { TaxDashboard } from "../components/organisms/TaxDashboard";
import { FinancialDashboard } from "../components/organisms/FinancialDashboard";
import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { Alert, Button, Heading, formatCalendarDate } from "@open-faktura/ui";
import {
  getOverview,
  InvoiceCalculationFailure,
  type Overview,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
export function OverviewPage() {
  const { companyId = "" } = useParams();
  const { t, language } = useAppTranslation();
  const { onExpired } = useWorkspace();
  const [result, setResult] = useState<{
      company: string;
      value: Overview;
    } | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const c = new AbortController();
    setError(false);
    getOverview(apiUrl, companyId, c.signal)
      .then((value) => {
        if (!c.signal.aborted) setResult({ company: companyId, value });
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          if (e instanceof InvoiceCalculationFailure && e.status === 401)
            onExpired();
          setError(true);
        }
      });
    return () => c.abort();
  }, [companyId, attempt, onExpired]);
  useEffect(() => {
    const refresh = () => setAttempt((v) => v + 1);
    window.addEventListener("focus", refresh);
    const timer = setInterval(refresh, 60000);
    return () => {
      window.removeEventListener("focus", refresh);
      clearInterval(timer);
    };
  }, []);
  const value = result?.company === companyId ? result.value : null;
  return (
    <div className="space-y-group">
      <Heading level={1} size="page">
        {t("navOverview")}
      </Heading>
      <p className="text-muted">{t("overviewIntro")}</p>
      {error && (
        <Alert tone="danger" role="alert">
          {t("requestFailed")}
          <Button variant="secondary" onClick={() => setAttempt((v) => v + 1)}>
            {t("retry")}
          </Button>
        </Alert>
      )}
      {!value && !error && <p role="status">{t("draftLoading")}</p>}
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
            refresh={attempt}
            onExpired={onExpired}
          />
          <TaxDashboard
            key={"tax-" + companyId}
            company={companyId}
            today={value.today}
            refresh={attempt}
            onExpired={onExpired}
          />
          <div className="grid gap-field sm:grid-cols-2 lg:grid-cols-4">
            {(
              [
                ["paymentRemaining", value.outstanding, "outstanding"],
                ["paymentOverdue", value.overdue, "overdue"],
                ["paymentReceived", value.received, "issued"],
                ["paymentOverpayment", value.overpayment, "overpaid"],
              ] as const
            ).map(([label, amount, status]) => (
              <Link
                key={label}
                to={`../invoices?status=${status}`}
                relative="path"
                className="workspace-panel p-field space-y-control hover:bg-sidebar"
              >
                <Heading level={2} size="card">
                  {t(label)}
                </Heading>
                <p
                  className={`text-2xl font-semibold tabular-nums ${status === "overdue" && value.overdue_count > 0 ? "text-danger" : ""}`}
                >
                  {moneyText(amount, language)}
                </p>
                <p className="text-xs text-muted">
                  {t("overviewOpenInvoices")}
                </p>
              </Link>
            ))}
          </div>
          <section className="workspace-panel p-field space-y-field">
            <Heading level={2} size="card">
              {t("overviewDocuments")}
            </Heading>
            <dl className="grid gap-field sm:grid-cols-2 lg:grid-cols-4">
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
                <div key={label}>
                  <dt className="text-sm text-muted">{t(label)}</dt>
                  <dd>
                    <Link
                      className="text-xl font-semibold underline underline-offset-4"
                      to={`../invoices?status=${status}`}
                      relative="path"
                    >
                      {count}
                    </Link>
                  </dd>
                </div>
              ))}
            </dl>
            <p className="text-sm text-muted">{t("overviewScope")}</p>
          </section>
        </>
      )}
    </div>
  );
}
