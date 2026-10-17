import { DatePicker, Field } from "@open-faktura/ui";
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
      <Field
        label={t("maturityDays")}
        hint={t("draftMaturityHint")}
        type="number"
        min={0}
        step={1}
        disabled={disabled || !issuedOn}
        value={days ?? ""}
        onChange={(event) => {
          if (!issuedOn) return;
          if (event.target.value === "") {
            onChange({ issued_on: issuedOn, due_on: null });
            return;
          }
          const next = event.target.valueAsNumber;
          const maximum = Math.floor(
            (Date.parse("9999-12-31T12:00:00Z") -
              Date.parse(`${issuedOn}T12:00:00Z`)) /
              86400000,
          );
          if (!Number.isSafeInteger(next) || next < 0 || next > maximum) return;
          const due_on = addDays(issuedOn, next);
          if (due_on.length === 10) onChange({ issued_on: issuedOn, due_on });
        }}
      />
      <DatePicker
        label={t("draftDue")}
        value={dueOn ?? ""}
        disabled={disabled}
        onChange={(value) =>
          onChange({ issued_on: issuedOn, due_on: value || null })
        }
      />
    </>
  );
}
