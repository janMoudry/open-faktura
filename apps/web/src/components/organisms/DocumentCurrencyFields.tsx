import { useEffect, useRef, useState } from "react";
import { lookupExchangeRate } from "@open-faktura/api-client";
import { apiUrl } from "../../services/status";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { Button, DatePicker, Field, Select } from "@open-faktura/ui";
import type { DraftData } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
type Currency = DraftData["calculation"]["currency"];
type Value = {
  currency: Currency;
  exchange_rate?: { rate: string; units: string };
};
export function DocumentCurrencyFields({
  value,
  onChange,
  disabled = false,
  documentDate,
}: {
  value: Value;
  onChange: (v: Value) => void;
  disabled?: boolean;
  documentDate?: string | null;
}) {
  const { t, language } = useAppTranslation();
  const { company } = useWorkspace();
  const [date, setDate] = useState(
    documentDate ??
      new Intl.DateTimeFormat("sv-SE", { timeZone: "Europe/Prague" }).format(
        new Date(),
      ),
  );
  const [loading, setLoading] = useState(false);
  const [published, setPublished] = useState("");
  const [failed, setFailed] = useState(false);
  const controller = useRef<AbortController | null>(null);
  useEffect(() => {
    controller.current?.abort();
    setLoading(false);
    setPublished("");
    setFailed(false);
    return () => controller.current?.abort();
  }, [company?.id, value.currency, date, disabled]);
  useEffect(() => {
    if (documentDate) setDate(documentDate);
  }, [documentDate]);
  function change(next: Value) {
    setPublished("");
    setFailed(false);
    onChange(next);
  }
  async function load() {
    if (!company) return;
    const request = new AbortController();
    controller.current = request;
    setLoading(true);
    setFailed(false);
    setPublished("");
    try {
      const rate = await lookupExchangeRate(
        apiUrl,
        company.id,
        value.currency,
        date,
        request.signal,
      );
      if (!request.signal.aborted) {
        onChange({ ...value, exchange_rate: rate.exchange_rate });
        setPublished(rate.published_on);
      }
    } catch {
      if (!request.signal.aborted) setFailed(true);
    } finally {
      if (!request.signal.aborted) setLoading(false);
    }
  }
  return (
    <div className="space-y-field">
      <div className="flex flex-wrap gap-field">
        <Select
          label={t("documentCurrency")}
          value={value.currency}
          disabled={disabled || loading}
          options={[
            "CZK",
            "EUR",
            "USD",
            "GBP",
            "CHF",
            "PLN",
            "HUF",
            "JPY",
            "SEK",
            "NOK",
            "DKK",
            "CAD",
            "AUD",
          ].map((c) => ({ value: c, label: c }))}
          onChange={(currency) =>
            change({
              currency: currency as Currency,
              exchange_rate:
                currency === "CZK" ? undefined : { rate: "", units: "1" },
            })
          }
        />
        {value.currency !== "CZK" && (
          <>
            <Field
              label={t("exchangeRate")}
              value={value.exchange_rate?.rate ?? ""}
              disabled={disabled || loading}
              inputMode="decimal"
              onChange={(e) =>
                change({
                  ...value,
                  exchange_rate: {
                    rate: e.target.value.replace(",", "."),
                    units: value.exchange_rate?.units ?? "1",
                  },
                })
              }
            />
            <Field
              label={t("exchangeUnits")}
              value={value.exchange_rate?.units ?? "1"}
              disabled={disabled || loading}
              inputMode="numeric"
              onChange={(e) =>
                change({
                  ...value,
                  exchange_rate: {
                    rate: value.exchange_rate?.rate ?? "",
                    units: e.target.value,
                  },
                })
              }
            />
          </>
        )}
      </div>
      {value.currency !== "CZK" && (
        <div className="space-y-control">
          <div className="flex flex-wrap items-end gap-field">
            <DatePicker
              label={t("exchangeDate")}
              value={date}
              onChange={setDate}
              disabled={disabled || loading}
            />
            <Button
              variant="secondary"
              disabled={disabled || loading || !date || !company}
              onClick={() => void load()}
            >
              {t(loading ? "exchangeLoading" : "exchangeLoad")}
            </Button>
          </div>
          {published && (
            <p role="status" className="text-sm text-muted">
              {t("exchangePublished", {
                date: new Intl.DateTimeFormat(
                  language === "cs" ? "cs-CZ" : "en-GB",
                  { timeZone: "UTC" },
                ).format(new Date(`${published}T00:00:00Z`)),
              })}
            </p>
          )}
          {failed && (
            <p role="alert" className="text-sm text-danger">
              {t("exchangeFailed")}
            </p>
          )}
        </div>
      )}
      {value.currency !== "CZK" && (
        <p className="text-sm text-muted">{t("exchangeHelp")}</p>
      )}
    </div>
  );
}
