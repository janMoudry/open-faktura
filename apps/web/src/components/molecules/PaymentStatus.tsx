import { Badge, Icon } from "@open-faktura/ui";
import type { Settlement } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
export function PaymentStatus({
  value,
  compact = false,
}: {
  value: Settlement;
  compact?: boolean;
}) {
  const { t } = useAppTranslation();
  const keys = {
    unpaid: "paymentUnpaid",
    partial: "paymentPartial",
    paid: "paymentPaid",
    overpaid: "paymentOverpaid",
  } as const;
  if (compact) {
    const label =
      t(keys[value.status]) +
      (value.overdue ? ` · ${t("paymentOverdue")}` : "");
    return (
      <span
        className={`inline-flex ${value.overdue ? "text-danger" : value.status === "paid" ? "text-success" : value.status === "partial" ? "text-warning" : "text-muted"}`}
        title={label}
      >
        <Icon
          name={
            value.overdue
              ? "warning"
              : value.status === "paid"
                ? "check"
                : "clock"
          }
        />
        <span className="sr-only">{label}</span>
      </span>
    );
  }
  return (
    <span className="inline-flex flex-wrap gap-inline">
      <Badge
        tone={
          value.status === "paid"
            ? "success"
            : value.status === "partial"
              ? "warning"
              : "neutral"
        }
      >
        {t(keys[value.status])}
      </Badge>
      {value.overdue && <Badge tone="danger">{t("paymentOverdue")}</Badge>}
    </span>
  );
}
