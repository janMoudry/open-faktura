import { useEffect, useState } from "react";
import { Alert, DatePicker, Select } from "@open-faktura/ui";
import {
  cashTaxPreview,
  type CashTaxInput,
  type CashTaxSnapshot,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
import { cashError } from "../../utils/cash";
import { moneyText } from "../../utils/invoice";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { SalesFailure } from "@open-faktura/api-client";
export function CashTaxFields({
  companyId,
  amount,
  value,
  onChange,
}: {
  companyId: string;
  amount: string;
  value: CashTaxInput;
  onChange: (v: CashTaxInput) => void;
}) {
  const { t, language } = useAppTranslation(),
    { onExpired } = useWorkspace();
  const key = JSON.stringify({ companyId, amount, value });
  const [result, setResult] = useState<{
    key: string;
    data?: CashTaxSnapshot;
    error?: ReturnType<typeof cashError>;
  } | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    const timer = setTimeout(() => {
      void cashTaxPreview(apiUrl, companyId, amount, value, controller.signal)
        .then((data) => {
          if (!controller.signal.aborted) setResult({ key, data });
        })
        .catch((e) => {
          if (controller.signal.aborted) return;
          setResult({ key, error: cashError(e) });
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        });
    }, 250);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [key, companyId, amount, value, onExpired]);
  const current = result?.key === key ? result : null;
  return (
    <div className="space-y-field">
      <Alert>{t("cashTaxHelp")}</Alert>
      <Select
        label={t("cashTaxPriceMode")}
        value={value.price_mode}
        onChange={(v) =>
          onChange({ ...value, price_mode: v as "net" | "gross" })
        }
        options={[
          { value: "net", label: t("cashTaxNetPrice") },
          { value: "gross", label: t("cashTaxGrossPrice") },
        ]}
      />
      <Select
        label={t("cashTaxRate")}
        value={value.vat_percent}
        onChange={(v) =>
          onChange({ ...value, vat_percent: v as CashTaxInput["vat_percent"] })
        }
        options={["0", "12", "21"].map((v) => ({ value: v, label: `${v} %` }))}
      />
      <DatePicker
        required
        label={t("cashTaxDate")}
        value={value.tax_on}
        onChange={(v) => onChange({ ...value, tax_on: v })}
      />
      <div aria-live="polite">
        {!current ? (
          <p role="status">{t("cashTaxCalculating")}</p>
        ) : current.error ? (
          <Alert tone="danger">{t(current.error)}</Alert>
        ) : (
          current.data && (
            <dl className="grid grid-cols-3 gap-control">
              {(["net", "vat", "gross"] as const).map((k) => (
                <div key={k}>
                  <dt className="text-sm text-muted">
                    {t(
                      k === "net"
                        ? "cashTaxNet"
                        : k === "vat"
                          ? "cashTaxVat"
                          : "cashTaxTotal",
                    )}
                  </dt>
                  <dd className="font-medium">
                    {moneyText(current.data!.amounts[k], language)}
                  </dd>
                </div>
              ))}
            </dl>
          )
        )}
      </div>
    </div>
  );
}
