import { useEffect, useRef, useState } from "react";
import { useParams } from "react-router";
import { Alert, Badge, Button, Heading, Select } from "@open-faktura/ui";
import {
  getPublicInvoice,
  publicInvoicePdf,
  recordInvoiceView,
  SalesFailure,
  type PublicInvoice,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
export function PublicInvoicePage() {
  const { token = "" } = useParams();
  return <Invoice key={token} token={token} />;
}
function Invoice({ token }: { token: string }) {
  const { t, language, setLanguage } = useAppTranslation();
  const [value, setValue] = useState<PublicInvoice | null>(null),
    [pdf, setPdf] = useState<string | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [attempt, setAttempt] = useState(0),
    [loading, setLoading] = useState(true);
  const visit = useRef(crypto.randomUUID()),
    viewed = useRef(false),
    initialLanguage = useRef(false);
  useEffect(() => {
    const abort = new AbortController();
    let url: string | undefined;
    setLoading(true);
    setError("");
    setPdf(null);
    setValue(null);
    async function load() {
      try {
        const v = await getPublicInvoice(apiUrl, token, abort.signal);
        const bytes = await publicInvoicePdf(apiUrl, token, abort.signal);
        if (abort.signal.aborted) return;
        url = URL.createObjectURL(bytes);
        setValue(v);
        setPdf(url);
        if (!initialLanguage.current) {
          setLanguage(v.invoice.language === "en" ? "en" : "cs");
          initialLanguage.current = true;
        }
      } catch (e) {
        if (!abort.signal.aborted)
          setError(
            e instanceof SalesFailure && e.status === 404
              ? "publicInvoiceUnavailable"
              : "publicInvoiceError",
          );
      } finally {
        if (!abort.signal.aborted) setLoading(false);
      }
    }
    void load();
    return () => {
      abort.abort();
      if (url) URL.revokeObjectURL(url);
    };
  }, [token, attempt]);
  useEffect(() => {
    if (!value || !pdf) return;
    const abort = new AbortController();
    let busy = false;
    const record = () => {
      if (document.visibilityState !== "visible" || viewed.current || busy)
        return;
      busy = true;
      recordInvoiceView(
        apiUrl,
        token,
        visit.current,
        value.revision,
        abort.signal,
      )
        .then(() => {
          viewed.current = true;
        })
        .catch(() => {})
        .finally(() => {
          busy = false;
        });
    };
    record();
    document.addEventListener("visibilitychange", record);
    return () => {
      abort.abort();
      document.removeEventListener("visibilitychange", record);
    };
  }, [value, pdf, token]);
  const statuses: Record<string, AppKey> = {
    paid: "paymentPaid",
    overpaid: "paymentOverpaid",
    partial: "paymentPartial",
    unpaid: "paymentUnpaid",
  };
  return (
    <div className="min-h-screen bg-canvas">
      <header className="border-b border-border bg-surface">
        <div className="mx-auto flex max-w-5xl flex-wrap items-center justify-between gap-field p-field">
          <span className="font-bold">openfaktura</span>
          <Select
            label={t("publicQuoteLanguage")}
            value={language}
            onChange={(v) => setLanguage(v === "en" ? "en" : "cs")}
            options={[
              { value: "cs", label: "Čeština" },
              { value: "en", label: "English" },
            ]}
          />
        </div>
      </header>
      <main className="mx-auto max-w-5xl space-y-field p-field">
        {loading && <p role="status">{t("draftLoading")}</p>}
        {error && (
          <Alert tone="warning" role="alert">
            <Heading level={1} size="page">
              {t(error)}
            </Heading>
            <p>{t("publicInvoiceUnavailableHint")}</p>
            <Button
              variant="secondary"
              onClick={() => setAttempt((v) => v + 1)}
            >
              {t("retry")}
            </Button>
          </Alert>
        )}
        {value && pdf && (
          <>
            <div className="flex flex-wrap items-center justify-between gap-field">
              <Heading level={1} size="page">
                {t("publicInvoiceTitle")} {value.number}
              </Heading>
              <Badge>{t(statuses[value.status] ?? "paymentUnpaid")}</Badge>
            </div>
            <p className="text-sm text-muted">{t("publicInvoiceTracking")}</p>
            <div className="flex flex-wrap gap-control">
              <a
                className="ui-button ui-button-primary ui-button-md"
                href={pdf}
                download={`${value.number}.pdf`}
              >
                {t("publicInvoiceDownload")}
              </a>
              <a
                className="ui-button ui-button-secondary ui-button-md"
                href={pdf}
                target="_blank"
                rel="noreferrer"
              >
                {t("publicQuotePrint")}
              </a>
              <Button variant="ghost" onClick={() => setAttempt((v) => v + 1)}>
                {t("publicInvoiceReload")}
              </Button>
            </div>
            <p className="text-sm text-muted">{t("publicInvoicePdfHint")}</p>
            {navigator.pdfViewerEnabled && (
              <div className="invoice-sheet p-field">
                <iframe
                  src={pdf}
                  title={t("publicInvoiceTitle")}
                  className="h-pdf-preview w-full"
                />
              </div>
            )}
          </>
        )}
      </main>
    </div>
  );
}
