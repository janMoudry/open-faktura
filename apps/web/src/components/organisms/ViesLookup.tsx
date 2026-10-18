import { useState } from "react";
import { Alert, Button, formatCalendarDate } from "@open-faktura/ui";
import {
  lookupVies,
  EU_VAT_PREFIXES,
  InvoiceCalculationFailure,
  type ViesObservation,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
export function ViesLookup({
  company,
  vatNumber,
  onExpired,
}: {
  company: string;
  vatNumber: string;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [busy, setBusy] = useState(false),
    [failed, setFailed] = useState(false),
    [value, setValue] = useState<ViesObservation | null>(null);
  const normalized = vatNumber.trim().toUpperCase();
  async function lookup() {
    setBusy(true);
    setFailed(false);
    setValue(null);
    try {
      setValue(await lookupVies(apiUrl, company, normalized));
    } catch (e) {
      if (e instanceof InvoiceCalculationFailure && e.status === 401)
        onExpired();
      else setFailed(true);
    } finally {
      setBusy(false);
    }
  }
  if (
    !EU_VAT_PREFIXES.includes(normalized.slice(0, 2)) ||
    !/^..[A-Z0-9+*]{1,20}$/.test(normalized)
  )
    return null;
  return (
    <div className="space-y-field">
      <Button
        size="sm"
        type="button"
        variant="secondary"
        loading={busy}
        onClick={lookup}
      >
        {t("viesLookup")}
      </Button>
      {failed && (
        <Alert tone="warning" role="alert">
          {t("viesUnavailable")}
        </Alert>
      )}
      {value && (
        <div role="status" className="space-y-control text-sm">
          <p className="font-semibold">
            {t(value.valid ? "viesValid" : "viesInvalid")}
          </p>
          <p>
            {t("viesDate", {
              date: formatCalendarDate(value.requested_on, language),
            })}
          </p>
          {value.name && <p>{value.name}</p>}
          {value.address && (
            <p className="whitespace-pre-line">{value.address}</p>
          )}
          <p className="text-muted">{t("viesScope")}</p>
        </div>
      )}
    </div>
  );
}
