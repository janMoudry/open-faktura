import { InvoiceSummary } from "../components/organisms/InvoiceSummary";
import { useEffect, useRef, useState } from "react";
import { useParams } from "react-router";
import {
  Alert,
  Badge,
  Button,
  Dialog,
  Heading,
  Select,
  Table,
  TableHead,
  TableHeader,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  getPublicQuote,
  publicQuotePdf,
  recordQuoteView,
  SalesFailure,
  type PublicQuote,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../locales";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
import { salesStates } from "../utils/sales";
export function PublicQuotePage() {
  const { token = "" } = useParams();
  return <Quote key={token} token={token} />;
}
function Quote({ token }: { token: string }) {
  const { t, language, setLanguage } = useAppTranslation();
  const [quote, setQuote] = useState<PublicQuote | null>(null),
    [error, setError] = useState(""),
    [attempt, setAttempt] = useState(0),
    [busy, setBusy] = useState(false),
    [preview, setPreview] = useState<string | null>(null);
  const visit = useRef(crypto.randomUUID()),
    viewed = useRef(false),
    blob = useRef<string | null>(null),
    initialLanguage = useRef(false);
  useEffect(() => {
    const abort = new AbortController();
    setQuote(null);
    setError("");
    getPublicQuote(apiUrl, token, abort.signal)
      .then((v) => {
        setQuote(v);
        if (!initialLanguage.current) {
          setLanguage(v.invoice.language === "en" ? "en" : "cs");
          initialLanguage.current = true;
        }
      })
      .catch((e) => {
        if (!abort.signal.aborted)
          setError(
            e instanceof SalesFailure && e.status === 404
              ? "missing"
              : "network",
          );
      });
    return () => abort.abort();
  }, [token, attempt]);
  useEffect(() => {
    if (!quote) return;
    const abort = new AbortController();
    let inFlight = false;
    const record = () => {
      if (document.visibilityState !== "visible" || viewed.current || inFlight)
        return;
      inFlight = true;
      recordQuoteView(
        apiUrl,
        token,
        visit.current,
        quote.revision,
        abort.signal,
      )
        .then(() => {
          viewed.current = true;
        })
        .catch(() => {})
        .finally(() => {
          inFlight = false;
        });
    };
    record();
    document.addEventListener("visibilitychange", record);
    return () => {
      abort.abort();
      document.removeEventListener("visibilitychange", record);
    };
  }, [quote, token]);
  useEffect(
    () => () => {
      if (blob.current) URL.revokeObjectURL(blob.current);
    },
    [],
  );
  async function pdf(print: boolean) {
    setBusy(true);
    try {
      const b = await publicQuotePdf(apiUrl, token);
      if (blob.current) URL.revokeObjectURL(blob.current);
      blob.current = URL.createObjectURL(b);
      if (print) setPreview(blob.current);
      else {
        const a = document.createElement("a");
        a.href = blob.current;
        a.download = `${quote?.number ?? "quotation"}.pdf`;
        a.click();
      }
    } catch (e) {
      setError(
        e instanceof SalesFailure && e.status === 404 ? "missing" : "network",
      );
      if (e instanceof SalesFailure && e.status === 404) {
        setQuote(null);
        setPreview(null);
      }
    } finally {
      setBusy(false);
    }
  }
  const date = (v: string | null) =>
    v
      ? new Intl.DateTimeFormat(language === "cs" ? "cs-CZ" : "en-GB").format(
          new Date(v + "T12:00:00Z"),
        )
      : "—";
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
        {error && (
          <Alert tone="warning" role="alert">
            <Heading level={1} size="page">
              {t(
                error === "missing"
                  ? "publicQuoteUnavailable"
                  : "publicQuoteError",
              )}
            </Heading>
            {error === "missing" && <p>{t("publicQuoteUnavailableHint")}</p>}
            <Button
              variant="secondary"
              onClick={() => setAttempt((v) => v + 1)}
            >
              {t("retry")}
            </Button>
          </Alert>
        )}
        {!quote && !error && <p role="status">{t("salesLoading")}</p>}
        {quote && (
          <>
            <div className="flex flex-wrap items-center justify-between gap-field">
              <div>
                <p className="text-sm text-muted">
                  {t("publicQuoteCurrent")} · {quote.revision}
                </p>
                <Heading level={1} size="page">
                  {t("publicQuoteTitle")} {quote.number}
                </Heading>
              </div>
              <Badge>{t(salesStates[quote.status] ?? "salesCreated")}</Badge>
            </div>
            <p className="text-sm text-muted">{t("publicQuoteTracking")}</p>
            <div className="flex flex-wrap gap-inline">
              <Button disabled={busy} onClick={() => void pdf(false)}>
                {t("publicQuoteDownload")}
              </Button>
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() => void pdf(true)}
              >
                {t("publicQuotePrint")}
              </Button>
              <Button variant="ghost" onClick={() => setAttempt((v) => v + 1)}>
                {t("publicQuoteReload")}
              </Button>
            </div>
            <article className="invoice-sheet">
              <div className="invoice-section">
                <Heading level={2} size="section">
                  {quote.name}
                </Heading>
                <div className="grid gap-group sm:grid-cols-2">
                  {(
                    [
                      [t("publicQuoteSupplier"), quote.invoice.supplier],
                      [t("publicQuoteCustomer"), quote.invoice.customer],
                    ] as const
                  ).map(([label, party]) => {
                    if (!party) return null;
                    return (
                      <section key={String(label)}>
                        <Heading level={3} size="card">
                          {String(label)}
                        </Heading>
                        <p>{party.name}</p>
                        <p>{party.address.street}</p>
                        <p>
                          {[
                            [party.address.postal_code, party.address.city]
                              .filter(Boolean)
                              .join(" "),
                            party.address.country,
                          ]
                            .filter(Boolean)
                            .join(", ")}
                        </p>
                        {party.registration_number && (
                          <p>
                            {t("registrationNumber")}:{" "}
                            {party.registration_number}
                          </p>
                        )}
                        {party.vat_number && <p>{party.vat_number}</p>}
                        {party.email && (
                          <p className="break-all">
                            {t("publicQuoteEmail")}: {party.email}
                          </p>
                        )}
                      </section>
                    );
                  })}
                </div>
                <dl className="grid gap-field sm:grid-cols-2">
                  <div>
                    <dt className="text-sm text-muted">
                      {t("publicQuoteIssued")}
                    </dt>
                    <dd>{date(quote.invoice.issued_on)}</dd>
                  </div>
                  <div>
                    <dt className="text-sm text-muted">
                      {t("publicQuoteValid")}
                    </dt>
                    <dd>{date(quote.invoice.due_on)}</dd>
                  </div>
                </dl>
              </div>
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("publicQuoteDescription")}</TableHead>
                      <TableHead>{t("publicQuoteQuantity")}</TableHead>
                      <TableHead className="text-right">
                        {t("publicQuotePrice")}
                      </TableHead>
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {quote.invoice.calculation.lines.map((line) => (
                      <TableRow key={line.id}>
                        <TableCell>{line.description}</TableCell>
                        <TableCell>
                          {line.quantity} {line.unit}
                        </TableCell>
                        <TableCell className="text-right whitespace-nowrap">
                          {moneyText(
                            quote.totals?.lines.find((v) => v.id === line.id)
                              ?.amounts.gross ?? "0",
                            language,
                            quote.invoice.calculation.currency,
                          )}
                        </TableCell>
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
              <InvoiceSummary
                calculation={quote.invoice.calculation}
                totals={quote.totals}
                totalLabel={t("quoteTotalPrice")}
              />
            </article>
          </>
        )}
        <Dialog
          open={!!preview}
          onClose={() => setPreview(null)}
          title={t("publicQuotePrint")}
          description={t("publicQuoteDownloadedHint")}
        >
          {preview && (
            <>
              <iframe
                title={t("publicQuotePrint")}
                src={preview}
                className="h-pdf-preview w-full"
              />
              <a
                className="ui-button ui-button-secondary ui-button-md"
                href={preview}
                target="_blank"
                rel="noreferrer"
              >
                {t("quoteOpenPdf")}
              </a>
            </>
          )}
        </Dialog>
      </main>
    </div>
  );
}
