import { useState } from "react";
import { DatePicker, Select } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
import { addDays } from "../../utils/invoice";

// Both inputs edit the existing calendar dates; days are never persisted separately.
export function InvoicePaymentDates({
  issuedOn,
  dueOn,
  disabled,
  onChange,
}: {
  issuedOn: string | null;
  dueOn: string | null;
  disabled: boolean;
  onChange: (dates: {
    issued_on: string | null;
    due_on: string | null;
  }) => void;
}) {
  const { t } = useAppTranslation();
  const [custom, setCustom] = useState(false);
  const presets = [7, 14, 21, 30, 60];
  const days =
    issuedOn && dueOn
      ? Math.round(
          (Date.parse(`${dueOn}T12:00:00Z`) -
            Date.parse(`${issuedOn}T12:00:00Z`)) /
            86400000,
        )
      : null;
  return (
    <>
      <DatePicker
        label={t("draftIssued")}
        value={issuedOn ?? ""}
        disabled={disabled}
        onChange={(value) => {
          const issued_on = value || null;
          onChange({
            issued_on,
            due_on:
              issued_on && days !== null ? addDays(issued_on, days) : dueOn,
          });
        }}
      />
      <Select
        label={t("draftMaturity")}
        hint={t("draftMaturityHint")}
        disabled={disabled || !issuedOn}
        value={
          !custom && days !== null && presets.includes(days)
            ? String(days)
            : "custom"
        }
        options={[
          ...presets.map((days) => ({
            value: String(days),
            label: t("draftMaturityDays", { days }),
          })),
          { value: "custom", label: t("draftMaturityCustom") },
        ]}
        onChange={(value) => {
          setCustom(value === "custom");
          if (!issuedOn || value === "custom") return;
          const next = Number(value);
          if (!presets.includes(next)) return;
          const due_on = addDays(issuedOn, next);
          if (due_on.length === 10) onChange({ issued_on: issuedOn, due_on });
        }}
      />
      <DatePicker
        label={t("draftDue")}
        value={dueOn ?? ""}
        disabled={disabled}
        onChange={(value) => {
          setCustom(true);
          onChange({ issued_on: issuedOn, due_on: value || null });
        }}
      />
    </>
  );
}
