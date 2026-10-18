import { Link } from "react-router";
import { Icon } from "@open-faktura/ui";
import type { Overview } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { moneyText } from "../../utils/invoice";
export function InvoiceOverviewMetrics({ value }: { value: Overview }) {
  const { t, language } = useAppTranslation();
  return (
    <div className="invoice-summary-strip">
      {(
        [
          ["paymentOutstanding", value.outstanding, "outstanding", "clock"],
          ["paymentOverdue", value.overdue, "overdue", "warning"],
          ["paymentReceived", value.received, "issued", "check"],
        ] as const
      ).map(([label, amount, status, icon]) => (
        <Link key={label} to={`?status=${status}`}>
          <p className="metric-label">
            <Icon name={icon} />
            {t(label)}
          </p>
          <p className="metric-value">{moneyText(amount, language)}</p>
          <p className="metric-hint">{t("overviewAllHistory")}</p>
        </Link>
      ))}
    </div>
  );
}
