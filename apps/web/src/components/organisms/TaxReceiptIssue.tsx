import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import {
  Alert,
  Button,
  DatePicker,
  Dialog,
  Field,
  Select,
} from "@open-faktura/ui";
import {
  previewTaxReceipt,
  issueTaxReceipt,
  SalesFailure,
  type TaxReceipt,
  type SalesDocument,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
import { salesError } from "../../utils/sales";
import { TaxReceiptSummary } from "./TaxReceiptSummary";
export function TaxReceiptIssue({
  document: d,
  paymentId,
  onClose,
}: {
  document: SalesDocument;
  paymentId: string;
  onClose: () => void;
}) {
  const { t } = useAppTranslation(),
    { onExpired } = useWorkspace(),
    navigate = useNavigate();
  const [preview, setPreview] = useState<TaxReceipt | null>(null),
    [error, setError] = useState<AppKey | "">(""),
    [busy, setBusy] = useState(false);
  const [date, setDate] = useState(
    new Intl.DateTimeFormat("en-CA", {
      timeZone: "Europe/Prague",
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    }).format(new Date()),
  );
  const [description, setDescription] = useState(t("taxDefaultDescription")),
    [language, setLanguage] = useState(d.data.invoice.language);
  const key = useRef(crypto.randomUUID()),
    last = useRef("");
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else setError(salesError(e));
  }
  useEffect(() => {
    let active = true;
    previewTaxReceipt(apiUrl, d.company_id, d.id, paymentId)
      .then((v) => {
        if (active) setPreview(v);
      })
      .catch((e) => {
        if (active) failure(e);
      });
    return () => {
      active = false;
    };
  }, [d.company_id, d.id, paymentId]);
  async function issue() {
    setBusy(true);
    setError("");
    try {
      const fields = {
        revision: d.revision,
        advance_id: d.id,
        payment_id: paymentId,
        issued_on: date,
        description,
        language: language as "cs" | "en",
      };
      const hash = JSON.stringify(fields);
      if (last.current && last.current !== hash)
        key.current = crypto.randomUUID();
      last.current = hash;
      const v = await issueTaxReceipt(apiUrl, d.company_id, {
        ...fields,
        request_id: key.current,
      });
      onClose();
      navigate(`/companies/${d.company_id}/tax-receipts/${v.id}`);
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <Dialog
      open
      onClose={() => {
        if (!busy) onClose();
      }}
      title={t("taxIssue")}
      description={t("taxIssueHelp")}
    >
      <form
        className="space-y-field"
        onSubmit={(e) => {
          e.preventDefault();
          void issue();
        }}
      >
        {error && (
          <Alert tone="danger" role="alert">
            {t(error as AppKey)}
          </Alert>
        )}
        {!preview && !error && <p role="status">{t("salesLoading")}</p>}
        {preview && (
          <>
            <TaxReceiptSummary document={preview} />
            <DatePicker
              label={t("taxIssuedOn")}
              value={date}
              onChange={setDate}
              disabled={busy}
            />
            <Field
              label={t("taxDescription")}
              value={description}
              required
              maxLength={500}
              onChange={(e) => setDescription(e.target.value)}
              disabled={busy}
            />
            <Select
              label={t("taxLanguage")}
              value={language}
              onChange={(v) => setLanguage(v as "cs" | "en")}
              disabled={busy}
              options={[
                { value: "cs", label: t("taxCzech") },
                { value: "en", label: t("taxEnglish") },
              ]}
            />
            <p className="text-sm text-muted">{t("taxAllocationHint")}</p>
            <Button type="submit" disabled={busy}>
              {t("taxIssue")}
            </Button>
          </>
        )}
      </form>
    </Dialog>
  );
}
