import { ViesLookup } from "./ViesLookup";
import { useState } from "react";
import { Alert, Button, formatCalendarDate } from "@open-faktura/ui";
import {
  lookupVatRegistry,
  InvoiceCalculationFailure,
  type VatObservation,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
const subjects: Record<VatObservation["subject_type"], AppKey> = {
  vat_payer: "vatRegistryPayer",
  vat_group: "vatRegistryGroup",
  identified_person: "vatRegistryIdentified",
  unreliable_person: "vatRegistryUnreliablePerson",
  not_found: "vatRegistryNotFound",
};
const reliability: Record<VatObservation["reliability"], AppKey> = {
  reliable: "vatRegistryReliable",
  unreliable: "vatRegistryUnreliable",
  not_found: "vatRegistryNotFound",
};
export function VatRegistryLookup({
  company,
  vatNumber,
  onExpired,
}: {
  company: string;
  vatNumber: string;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<VatObservation | null>(null);
  const [failed, setFailed] = useState(false);
  async function lookup() {
    setBusy(true);
    setFailed(false);
    setResult(null);
    try {
      setResult(await lookupVatRegistry(apiUrl, company, vatNumber));
    } catch (e) {
      if (e instanceof InvoiceCalculationFailure && e.status === 401)
        onExpired();
      else setFailed(true);
    } finally {
      setBusy(false);
    }
  }
  if (!/^CZ\d{1,10}$/i.test(vatNumber.trim()))
    return (
      <ViesLookup
        key={vatNumber}
        company={company}
        vatNumber={vatNumber}
        onExpired={onExpired}
      />
    );
  return (
    <div className="space-y-field">
      <Button
        type="button"
        variant="secondary"
        size="sm"
        loading={busy}
        onClick={lookup}
      >
        {t("vatRegistryLookup")}
      </Button>
      {failed && (
        <Alert tone="warning" role="alert">
          {t("vatRegistryUnavailable")}
        </Alert>
      )}
      {result && (
        <div role="status" className="space-y-control text-sm">
          <p className="font-semibold">
            {t(subjects[result.subject_type])} ·{" "}
            {t(reliability[result.reliability])}
          </p>
          <p>
            {t("vatRegistryGenerated", {
              date: formatCalendarDate(result.generated_on, language),
            })}
          </p>
          <p className="text-muted">{t("vatRegistryScope")}</p>
          {result.accounts.length > 0 && (
            <ul className="list-disc pl-group">
              {result.accounts.map((a, i) => (
                <li key={i}>
                  {a.number} ·{" "}
                  {t("vatRegistryPublished", {
                    date: formatCalendarDate(a.published_on, language),
                  })}
                  {a.ended_on && (
                    <>
                      {" "}
                      ·{" "}
                      {t("vatRegistryEnded", {
                        date: formatCalendarDate(a.ended_on, language),
                      })}
                    </>
                  )}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
