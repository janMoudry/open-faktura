import { unitOptions } from "../../utils/units";
import { useRef, type ReactNode } from "react";
import { Button, Field, Heading, Select, Icon } from "@open-faktura/ui";
import type { DraftData, InvoiceTotals } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { moneyText, newLine } from "../../utils/invoice";
export function InvoiceLines({
  value,
  onChange,
  disabled,
  errors,
  totals,
  catalogPicker,
  title,
}: {
  value: DraftData["calculation"];
  onChange: (v: DraftData["calculation"]) => void;
  disabled: boolean;
  errors: string[];
  totals: InvoiceTotals | null;
  catalogPicker?: ReactNode;
  title?: string;
}) {
  const { t, language } = useAppTranslation();
  const root = useRef<HTMLDivElement>(null);
  const units = unitOptions(t);
  const patch = (index: number, patch: Partial<(typeof value.lines)[number]>) =>
    onChange({
      ...value,
      lines: value.lines.map((l, i) => (i === index ? { ...l, ...patch } : l)),
    });
  return (
    <section className="invoice-section" ref={root}>
      <div className="flex flex-wrap items-center justify-between gap-field">
        <Heading level={2} size="card">
          {title ?? t("draftItems")}
        </Heading>
        {value.tax_mode === "vat_payer" && (
          <Select
            label={t("draftPriceMode")}
            value={value.price_mode}
            disabled={disabled}
            options={[
              { value: "net", label: t("draftNetPrice") },
              { value: "gross", label: t("draftGrossPrice") },
            ]}
            onChange={(price_mode) =>
              onChange({ ...value, price_mode: price_mode as "net" | "gross" })
            }
          />
        )}
      </div>
      {!disabled && catalogPicker}
      <div className="invoice-line-head" aria-hidden="true">
        <span>{t("draftDescription")}</span>
        <span>{t("draftUnitPrice")}</span>
        <span>{t("draftQuantity")}</span>
        <span>{t("draftUnit")}</span>
        <span className="text-right">{t("invoiceColumnAmount")}</span>
        <span>{t("invoiceListActions")}</span>
      </div>
      {value.lines.map((line, index) => (
        <fieldset key={line.id} className="invoice-editor-line">
          <legend className="sr-only">
            {t("draftItem", { number: index + 1 })}
          </legend>
          <div className="invoice-line-input invoice-line-description">
            <Field
              data-line-description
              label={t("draftDescription")}
              value={line.description}
              readOnly={disabled}
              maxLength={2000}
              onChange={(e) => patch(index, { description: e.target.value })}
            />
            {line.catalog_id && (
              <p className="text-sm text-muted">{t("stockInvoiceLink")}</p>
            )}
          </div>
          {(
            [
              ["unit_price", "draftUnitPrice"],
              ["quantity", "draftQuantity"],
              ["unit", "draftUnit"],
            ] as const
          ).map(([key, label]) => (
            <div className="invoice-line-input" key={key}>
              {key === "unit" ? (
                <Select
                  key={key}
                  label={t(label)}
                  value={line.unit}
                  disabled={disabled}
                  options={
                    units.some((u) => u.value === line.unit)
                      ? units
                      : [...units, { value: line.unit, label: line.unit }]
                  }
                  onChange={(unit) => patch(index, { unit })}
                />
              ) : (
                <Field
                  key={key}
                  label={t(label)}
                  inputMode="decimal"
                  value={line[key]}
                  readOnly={disabled}
                  error={
                    line[key] !== "" && errors.includes(`lines.${index}.${key}`)
                      ? t("draftNumberError")
                      : undefined
                  }
                  onChange={(e) =>
                    patch(index, {
                      [key]: e.target.value.replace(",", "."),
                    })
                  }
                />
              )}
            </div>
          ))}
          <div className="invoice-line-total text-right text-sm font-semibold tabular-nums">
            <span className="lg:sr-only text-muted font-normal">
              {t("invoiceColumnAmount")}{" "}
            </span>
            {totals?.lines[index]
              ? moneyText(totals.lines[index].amounts.gross, language)
              : "—"}
          </div>
          {!disabled && (
            <Button
              variant="ghost"
              aria-label={t("draftRemoveItem")}
              title={t("draftRemoveItem")}
              onClick={() =>
                onChange({
                  ...value,
                  lines: value.lines.filter((l) => l.id !== line.id),
                })
              }
            >
              <Icon name="trash" />
            </Button>
          )}
          <div className="invoice-line-options flex flex-wrap items-start gap-field">
            {value.tax_mode === "vat_payer" && (
              <Select
                label={t("draftVat")}
                value={line.vat_percent}
                disabled={disabled}
                options={["0", "12", "21"].map((v) => ({
                  value: v,
                  label: `${v} %`,
                }))}
                onChange={(vat_percent) =>
                  patch(index, {
                    vat_percent: vat_percent as "0" | "12" | "21",
                  })
                }
              />
            )}
            <details open={line.discount_percent !== "0" || undefined}>
              <summary className="cursor-pointer text-sm text-muted">
                {t("draftLineDiscount")}
              </summary>
              <Field
                label={t("draftDiscount")}
                inputMode="decimal"
                value={line.discount_percent}
                readOnly={disabled}
                error={
                  line.discount_percent !== "" &&
                  errors.includes(`lines.${index}.discount_percent`)
                    ? t("draftNumberError")
                    : undefined
                }
                onChange={(e) =>
                  patch(index, {
                    discount_percent: e.target.value.replace(",", "."),
                  })
                }
              />
            </details>
          </div>
        </fieldset>
      ))}
      {!disabled && (
        <Button
          type="button"
          variant="secondary"
          disabled={value.lines.length >= 500}
          onClick={() => {
            onChange({
              ...value,
              lines: [...value.lines, newLine(value.tax_mode)],
            });
            requestAnimationFrame(() =>
              root.current
                ?.querySelectorAll<HTMLInputElement>("[data-line-description]")
                .item(value.lines.length)
                ?.focus(),
            );
          }}
        >
          <Icon name="plus" />
          {t("draftAddItem")}
        </Button>
      )}
      <details
        className="max-w-md ml-auto"
        open={value.discount_percent !== "0" || undefined}
      >
        <summary className="cursor-pointer text-sm text-muted">
          {t("draftDocumentDiscount")}
        </summary>
        <div className="mt-field">
          <Field
            label={t("draftDiscount")}
            inputMode="decimal"
            value={value.discount_percent}
            readOnly={disabled}
            error={
              errors.includes("discount_percent")
                ? t("draftNumberError")
                : undefined
            }
            onChange={(e) =>
              onChange({
                ...value,
                discount_percent: e.target.value.replace(",", "."),
              })
            }
          />
        </div>
      </details>
    </section>
  );
}
