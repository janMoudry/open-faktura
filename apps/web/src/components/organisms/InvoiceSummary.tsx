import { Alert } from "@open-faktura/ui";
import type { DraftData, InvoiceTotals } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { moneyText } from "../../utils/invoice";
/** Shared presentation of server-calculated totals for draft and issued document. */
export function InvoiceSummary({
  calculation,
  totals,
  loading = false,
  failed = false,
  totalLabel,
}: {
  calculation: DraftData["calculation"];
  totals: Omit<InvoiceTotals, "calculation_version"> | null;
  loading?: boolean;
  failed?: boolean;
  totalLabel?: string;
}) {
  const { t, language } = useAppTranslation();
  return (
    <section
      className="invoice-section"
      aria-label={t("draftSummary")}
      aria-busy={loading}
    >
      <div className="max-w-md ml-auto space-y-control text-sm tabular-nums">
        {loading ? (
          <p role="status">{t("draftCalculating")}</p>
        ) : failed || !totals ? (
          <Alert tone="warning" role="status">
            {t("draftCalculationFailed")}
          </Alert>
        ) : (
          <>
            {/[1-9]/.test(calculation.discount_percent) && (
              <p className="text-right">
                {t("draftDocumentDiscount")}: {calculation.discount_percent} %
              </p>
            )}
            {calculation.tax_mode === "vat_payer" ? (
              <dl className="space-y-control">
                {totals.tax_summary.map((row) => (
                  <div
                    className="flex justify-between gap-inline"
                    key={row.vat_percent}
                  >
                    <dt>
                      {t("draftVat")} {row.vat_percent} % ·{" "}
                      {moneyText(row.amounts.net, language)}
                    </dt>
                    <dd>{moneyText(row.amounts.vat, language)}</dd>
                  </div>
                ))}
              </dl>
            ) : (
              <p className="text-right text-muted">{t("invoiceNonPayer")}</p>
            )}
            {/[1-9]/.test(totals.rounding) && (
              <div className="flex justify-between gap-inline">
                <span>{t("draftRounding")}</span>
                <span>{moneyText(totals.rounding, language)}</span>
              </div>
            )}
            <dl className="border-t border-border pt-field text-xl font-semibold">
              <div className="flex justify-between gap-field">
                <dt>{totalLabel ?? t("draftTotal")}</dt>
                <dd className="whitespace-nowrap">
                  {moneyText(totals.payable, language)}
                </dd>
              </div>
            </dl>
          </>
        )}
      </div>
    </section>
  );
}
