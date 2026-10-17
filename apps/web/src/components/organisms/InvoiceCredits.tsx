import { useEffect, useRef, useState } from "react";
import { Link, useNavigate } from "react-router";
import { Alert, Button, Heading } from "@open-faktura/ui";
import {
  createCredit,
  listCredits,
  SalesFailure,
  type CreditPage,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
import { salesError } from "../../utils/sales";
import { moneyText } from "../../utils/invoice";
export function InvoiceCredits({
  company,
  invoice,
  canCreate,
}: {
  company: string;
  invoice: string;
  canCreate: boolean;
}) {
  const { t, language } = useAppTranslation(),
    { onExpired } = useWorkspace(),
    navigate = useNavigate();
  const [page, setPage] = useState<CreditPage | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [busy, setBusy] = useState(false);
  const key = useRef(crypto.randomUUID());
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else
      setError(
        e instanceof SalesFailure && e.fields.includes("credit_amount")
          ? "creditInvalid"
          : salesError(e),
      );
  }
  useEffect(() => {
    const c = new AbortController();
    setPage(null);
    listCredits(apiUrl, company, "", 1, invoice, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setPage(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) failure(e);
      });
    return () => c.abort();
  }, [company, invoice]);
  async function create() {
    setBusy(true);
    setError("");
    try {
      const v = await createCredit(apiUrl, company, invoice, key.current);
      navigate(`/companies/${company}/credit-notes/${v.id}`);
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="workspace-panel p-field space-y-field">
      <div className="flex flex-wrap items-center justify-between gap-field">
        <Heading level={2} size="section">
          {t("navCreditNotes")}
        </Heading>
        {canCreate && (
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => void create()}
          >
            {t("creditCreate")}
          </Button>
        )}
      </div>
      {error && (
        <Alert tone="danger" role="alert">
          {t(error)}
        </Alert>
      )}
      <p className="text-sm text-muted">{t("creditSourceHelp")}</p>
      {page?.items.map((d) => (
        <div key={d.id} className="flex flex-wrap justify-between gap-field">
          <Link
            className="underline"
            to={`/companies/${company}/credit-notes/${d.id}`}
          >
            {d.number ?? t("creditDraft")}
          </Link>
          <span>
            {t(
              d.status === "draft"
                ? "creditDraft"
                : d.status === "cancelled"
                  ? "creditCancelled"
                  : "creditIssued",
            )}{" "}
            · {moneyText(d.totals.payable, language)}
          </span>
          <span>
            {t("creditOffset")}: {moneyText(d.offset, language)}
          </span>
        </div>
      ))}
    </section>
  );
}
