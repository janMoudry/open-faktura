import { useEffect, useRef, useState } from "react";
import { Alert, Button, Dialog, Select, Icon } from "@open-faktura/ui";
import {
  cashPdf,
  SalesFailure,
  type CashMovement,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
export function CashDocumentOutput({
  companyId,
  movement,
}: {
  companyId: string;
  movement: CashMovement;
}) {
  const { t } = useAppTranslation(),
    { onExpired } = useWorkspace();
  const [language, setLanguage] = useState<"cs" | "en">("cs"),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(false),
    [preview, setPreview] = useState("");
  const url = useRef("");
  useEffect(
    () => () => {
      if (url.current) URL.revokeObjectURL(url.current);
    },
    [],
  );
  async function pdf(print: boolean) {
    setBusy(true);
    setError(false);
    try {
      const blob = await cashPdf(apiUrl, companyId, movement.id, language);
      if (url.current) URL.revokeObjectURL(url.current);
      url.current = URL.createObjectURL(blob);
      if (print) setPreview(url.current);
      else {
        const a = document.createElement("a");
        a.href = url.current;
        a.download = `${movement.number}-${language}.pdf`;
        a.click();
      }
    } catch (e) {
      setError(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="space-y-field">
      <div className="flex flex-wrap items-end gap-control">
        <Select
          label={t("cashPdfLanguage")}
          value={language}
          disabled={busy}
          onChange={(v) => setLanguage(v as "cs" | "en")}
          options={[
            { value: "cs", label: t("taxCzech") },
            { value: "en", label: t("taxEnglish") },
          ]}
        />
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() => void pdf(false)}
        >
          <Icon name="download" />
          {t("cashPdf")}
        </Button>
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() => void pdf(true)}
        >
          <Icon name="print" />
          {t("cashPrint")}
        </Button>
      </div>
      {error && <Alert tone="danger">{t("cashPdfError")}</Alert>}
      <Dialog
        open={!!preview}
        onClose={() => setPreview("")}
        title={t("cashPrint")}
        description={t("quotePrintHint")}
      >
        {preview && (
          <div className="space-y-field">
            <iframe
              title={t("cashPrint")}
              src={preview}
              className="h-pdf-preview w-full rounded-control border border-border"
            />
            <a
              className="ui-button ui-button-secondary ui-button-md"
              href={preview}
              target="_blank"
              rel="noreferrer"
            >
              {t("quoteOpenPdf")}
            </a>
          </div>
        )}
      </Dialog>
    </div>
  );
}
