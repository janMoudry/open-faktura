import type { ReactNode } from "react";
import { Field, Heading, Select } from "@open-faktura/ui";
import type { CatalogData } from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { unitOptions } from "../../utils/units";
export function CatalogFields({
  value,
  onChange,
  disabled,
  fields,
  footer,
}: {
  value: CatalogData;
  onChange: (value: CatalogData) => void;
  disabled: boolean;
  fields: string[];
  footer?: ReactNode;
}) {
  const { t } = useAppTranslation();
  const patch = (data: Partial<CatalogData>) => onChange({ ...value, ...data });
  const field = (
    key: "name" | "code" | "barcode" | "quantity" | "unit_price",
    label: AppKey,
  ) => (
    <Field
      horizontal
      label={t(label)}
      value={value[key]}
      readOnly={disabled}
      required={key === "name"}
      maxLength={
        key === "name" ? 200 : key === "code" || key === "barcode" ? 80 : 24
      }
      inputMode={
        key === "quantity" || key === "unit_price" ? "decimal" : "text"
      }
      error={fields.includes(key) ? t("catalogInvalidField") : undefined}
      onChange={(e) =>
        patch({
          [key]:
            key === "quantity" || key === "unit_price"
              ? e.target.value.replace(",", ".")
              : e.target.value,
        })
      }
    />
  );
  return (
    <div className="workspace-panel">
      <div className="p-form-inset space-y-field max-w-3xl catalog-fields">
        <Heading level={2} size="card">
          {t("catalogIdentity")}
        </Heading>
        <div className="space-y-inline">
          {field("name", "catalogName")}
          {field("code", "catalogCode")}
          {field("barcode", "catalogBarcode")}
          {field("quantity", "catalogQuantity")}
          <Select
            label={t("draftUnit")}
            value={value.unit}
            disabled={disabled}
            options={[
              ...unitOptions(t),
              ...(!unitOptions(t).some((o) => o.value === value.unit)
                ? [{ value: value.unit, label: value.unit }]
                : []),
            ]}
            onChange={(unit) => patch({ unit })}
          />
        </div>
        <Heading level={2} size="card">
          {t("catalogPricing")}
        </Heading>
        <div className="space-y-inline">
          {field("unit_price", "catalogPrice")}
          <Select
            label={t("draftPriceMode")}
            disabled={disabled}
            value={value.price_mode}
            options={[
              { value: "net", label: t("draftNetPrice") },
              { value: "gross", label: t("draftGrossPrice") },
            ]}
            onChange={(price_mode) =>
              patch({ price_mode: price_mode as "net" | "gross" })
            }
          />
          <Select
            label={t("draftVat")}
            disabled={disabled}
            value={value.vat_percent}
            options={["0", "12", "21"].map((value) => ({
              value,
              label: `${value} %`,
            }))}
            onChange={(vat_percent) =>
              patch({ vat_percent: vat_percent as CatalogData["vat_percent"] })
            }
          />
          <Field horizontal label={t("catalogCurrency")} value="CZK" readOnly />
        </div>
      </div>
      {footer}
    </div>
  );
}
