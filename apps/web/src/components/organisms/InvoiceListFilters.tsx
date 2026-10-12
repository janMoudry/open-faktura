import { useState } from "react";
import { Alert, Button, DatePicker, Field, Select } from "@open-faktura/ui";
import { InvoiceCustomerPicker } from "./InvoiceCustomerPicker";
import { useAppTranslation } from "../../locales";
export function InvoiceListFilters({
  company,
  params,
  onExpired,
  onChange,
}: {
  company: string;
  params: URLSearchParams;
  onExpired: () => void;
  onChange: (patch: Record<string, string>) => void;
}) {
  const { t } = useAppTranslation();
  const [dateField, setDateField] = useState(
    params.get("date_field") === "due_on" ? "due_on" : "issued_on",
  );
  const [from, setFrom] = useState(params.get("date_from") || ""),
    [to, setTo] = useState(params.get("date_to") || "");
  const [min, setMin] = useState(params.get("amount_min") || ""),
    [max, setMax] = useState(params.get("amount_max") || "");
  const [invalid, setInvalid] = useState(false);
  return (
    <form
      id="invoice-filters"
      className="invoice-sheet mt-control p-field space-y-field"
      onSubmit={(e) => {
        e.preventDefault();
        const amount = (v: string) => v.trim().replace(",", ".");
        const validAmount = (v: string) =>
          !v || /^\d{1,17}(\.\d{1,2})?$/.test(amount(v));
        const cents = (v: string) => {
          const [whole, part = ""] = amount(v).split(".");
          return BigInt(whole) * 100n + BigInt(part.padEnd(2, "0"));
        };
        if (
          !validAmount(min) ||
          !validAmount(max) ||
          (from && to && from > to) ||
          (min && max && cents(min) > cents(max))
        ) {
          setInvalid(true);
          return;
        }
        setInvalid(false);
        onChange({
          date_field: dateField,
          date_from: from,
          date_to: to,
          amount_min: amount(min),
          amount_max: amount(max),
        });
      }}
    >
      <div className="grid gap-group lg:grid-cols-3">
        <div className="space-y-control">
          <Select
            label={t("invoiceFilterDate")}
            value={dateField}
            options={[
              { value: "issued_on", label: t("draftIssued") },
              { value: "due_on", label: t("draftDue") },
            ]}
            onChange={setDateField}
          />
          <div className="grid grid-cols-2 gap-control">
            <DatePicker
              label={t("invoiceFilterFrom")}
              value={from}
              onChange={setFrom}
            />
            <DatePicker
              label={t("invoiceFilterTo")}
              value={to}
              onChange={setTo}
            />
          </div>
        </div>
        <div className="space-y-control">
          <InvoiceCustomerPicker
            company={company}
            disabled={false}
            onExpired={onExpired}
            onSelect={(c) =>
              onChange({ customer: c.id, customerName: c.data.name })
            }
          />
          <Select
            label={t("invoiceFilterStatus")}
            value={params.get("status") || "all"}
            options={(
              [
                ["all", "invoiceAll"],
                ["draft", "draftTab"],
                ["issued", "invoiceIssuedStatus"],
                ["outstanding", "paymentOutstanding"],
                ["paid", "paymentPaid"],
                ["partial", "paymentPartial"],
                ["overpaid", "paymentOverpaid"],
                ["overdue", "paymentOverdue"],
              ] as const
            ).map(([value, label]) => ({ value, label: t(label) }))}
            onChange={(status) => onChange({ status })}
          />
        </div>
        <div className="grid grid-cols-2 items-start gap-control">
          <Field
            label={t("invoiceAmountMin")}
            value={min}
            inputMode="decimal"
            onChange={(e) => setMin(e.target.value)}
          />
          <Field
            label={t("invoiceAmountMax")}
            value={max}
            inputMode="decimal"
            onChange={(e) => setMax(e.target.value)}
          />
        </div>
      </div>
      {invalid && (
        <Alert tone="danger" role="alert">
          {t("invoiceFilterInvalid")}
        </Alert>
      )}
      <div className="flex justify-end gap-control">
        <Button
          variant="ghost"
          onClick={() => {
            setFrom("");
            setTo("");
            setMin("");
            setMax("");
            setInvalid(false);
            onChange({
              customer: "",
              customerName: "",
              date_from: "",
              date_to: "",
              amount_min: "",
              amount_max: "",
              status: "all",
            });
          }}
        >
          {t("draftClearAll")}
        </Button>
        <Button type="submit">{t("invoiceApplyFilters")}</Button>
      </div>
    </form>
  );
}
