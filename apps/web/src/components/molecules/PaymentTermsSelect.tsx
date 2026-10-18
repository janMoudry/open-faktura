import { useState } from "react";
import { Field, Select } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";

export const paymentTermDays = [0, 7, 14, 21, 30, 60];

export function PaymentTermsSelect({
  value,
  onChange,
  label,
  hint,
  error,
  disabled,
  customLabel,
}: {
  value: number | null;
  onChange: (days: number | null) => void;
  label: string;
  hint?: string;
  error?: string;
  disabled?: boolean;
  customLabel?: string;
}) {
  const { t } = useAppTranslation();
  return (
    <Select
      label={label}
      hint={hint}
      error={error}
      disabled={disabled}
      value={
        value !== null && paymentTermDays.includes(value)
          ? String(value)
          : "custom"
      }
      options={[
        ...paymentTermDays.map((days) => ({
          value: String(days),
          label:
            days === 0
              ? t("draftMaturityImmediate")
              : t("draftMaturityDays", { days }),
        })),
        { value: "custom", label: customLabel ?? t("draftMaturityCustom") },
      ]}
      onChange={(value) => onChange(value === "custom" ? null : Number(value))}
    />
  );
}

export function CompanyPaymentTerms({
  value,
  onChange,
  disabled,
  error,
}: {
  value: number;
  onChange: (days: number) => void;
  disabled: boolean;
  error?: string;
}) {
  const { t } = useAppTranslation();
  const [custom, setCustom] = useState(false);
  const isCustom = custom || !paymentTermDays.includes(value);
  return (
    <div className="space-y-inline">
      <PaymentTermsSelect
        label={t("companyDefaultMaturity")}
        hint={t("companyDefaultMaturityHint")}
        customLabel={t("companyCustomMaturity")}
        value={isCustom ? null : value}
        disabled={disabled}
        error={error}
        onChange={(days) => {
          setCustom(days === null);
          if (days !== null) onChange(days);
        }}
      />
      {isCustom && (
        <Field
          label={t("maturityDays")}
          type="number"
          min={0}
          max={365}
          step={1}
          disabled={disabled}
          value={value}
          error={error}
          onChange={(event) =>
            onChange(event.target.value === "" ? 0 : Number(event.target.value))
          }
        />
      )}
    </div>
  );
}
