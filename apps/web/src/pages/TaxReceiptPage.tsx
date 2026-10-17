import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { Alert, Button, Heading, formatCalendarDate } from "@open-faktura/ui";
import {
  getTaxReceipt,
  taxReceiptPdf,
  SalesFailure,
  type TaxReceipt,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
import { salesError } from "../utils/sales";
import { TaxReceiptSummary } from "../components/organisms/TaxReceiptSummary";
export function TaxReceiptPage() {
  const { companyId = "", taxId = "" } = useParams(),
    { onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [d, setD] = useState<TaxReceipt | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [busy, setBusy] = useState(false),
    [attempt, setAttempt] = useState(0);
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else setError(salesError(e));
  }
  useEffect(() => {
    const c = new AbortController();
    setD(null);
    setError("");
    getTaxReceipt(apiUrl, companyId, taxId, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setD(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) failure(e);
      });
    return () => c.abort();
  }, [companyId, taxId, attempt]);
  async function pdf() {
    setBusy(true);
    setError("");
    try {
      const blob = await taxReceiptPdf(apiUrl, companyId, taxId),
        url = URL.createObjectURL(blob),
        a = document.createElement("a");
      a.href = url;
      a.download = `${d?.number}.pdf`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="space-y-section">
      <Link
        className="text-sm underline"
        to={`/companies/${companyId}/tax-receipts`}
      >
        {t("taxReceipts")}
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-field">
        <Heading level={1} size="page">
          {d?.number ?? t("taxReceipt")}
        </Heading>
        {d && (
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => void pdf()}
          >
            {t("taxPdf")}
          </Button>
        )}
      </div>
      {error && (
        <Alert tone="danger" role="alert">
          {t(error)}
          <Button variant="ghost" onClick={() => setAttempt((x) => x + 1)}>
            {t("retry")}
          </Button>
        </Alert>
      )}
      {!d && !error && <p role="status">{t("salesLoading")}</p>}
      {d && (
        <article className="invoice-sheet">
          <section className="invoice-section space-y-field">
            <Heading level={2} size="section">
              {t("taxReceipt")}
            </Heading>
            <p className="text-sm text-muted">{t("taxImmutable")}</p>
            <dl className="invoice-facts">
              <div>
                <dt>{t("taxAdvance")}</dt>
                <dd>
                  <Link
                    className="underline"
                    to={`/companies/${companyId}/advances/${d.advance_id}`}
                  >
                    {d.advance_number}
                  </Link>
                </dd>
              </div>
              <div>
                <dt>{t("taxIssuedOn")}</dt>
                <dd>{formatCalendarDate(d.data.issued_on ?? "", language)}</dd>
              </div>
              <div>
                <dt>{t("taxPaidOn")}</dt>
                <dd>{formatCalendarDate(d.data.tax_on ?? "", language)}</dd>
              </div>
              {[
                [t("taxSupplier"), d.data.supplier],
                [t("draftCustomer"), d.data.customer],
              ].map(([label, party]) => {
                if (!party || typeof party === "string") return null;
                return (
                  <div key={String(label)}>
                    <dt>{String(label)}</dt>
                    <dd>
                      {party.name}
                      <br />
                      {[
                        party.address.street,
                        `${party.address.postal_code} ${party.address.city}`.trim(),
                        party.address.country,
                      ]
                        .filter(Boolean)
                        .join(", ")}
                      <br />
                      {party.vat_number}
                    </dd>
                  </div>
                );
              })}
              <div>
                <dt>{t("taxDescription")}</dt>
                <dd>{d.data.calculation.lines[0]?.description}</dd>
              </div>
            </dl>
          </section>
          <section className="invoice-section">
            <TaxReceiptSummary document={d} />
          </section>
        </article>
      )}
    </div>
  );
}
