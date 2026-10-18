import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  DatePicker,
  Dialog,
  Input,
  Select,
} from "@open-faktura/ui";
import type { CashAccount, CashTaxInput } from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { useCashCommand } from "../../hooks/useCashCommand";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { CashTaxFields } from "./CashTaxFields";
export type CashForm =
  { kind: "account"; account?: CashAccount } | { kind: "manual" };
export function CashForms({
  form,
  accounts,
  companyId,
  onClose,
  onSaved,
}: {
  form: CashForm;
  accounts: CashAccount[];
  companyId: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useAppTranslation(),
    { setPageDirty, company } = useWorkspace();
  const { execute, busy, error } = useCashCommand(companyId);
  const existing = form.kind === "account" ? form.account : undefined;
  const today = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Europe/Prague",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());
  const [name, setName] = useState(existing?.data.name ?? ""),
    [opening, setOpening] = useState(existing?.data.opening_balance ?? "0.00"),
    [openingOn, setOpeningOn] = useState(existing?.data.opening_on ?? today),
    [archived, setArchived] = useState(existing?.archived ?? false);
  const [account, setAccount] = useState(
      accounts.find((a) => !a.archived)?.id ?? "",
    ),
    [date, setDate] = useState(today),
    [amount, setAmount] = useState(""),
    [direction, setDirection] = useState("income"),
    [vs, setVs] = useState(""),
    [partner, setPartner] = useState(""),
    [person, setPerson] = useState(""),
    [description, setDescription] = useState("");
  const [taxEnabled, setTaxEnabled] = useState(false);
  const [tax, setTax] = useState<CashTaxInput>({
    price_mode: "gross",
    vat_percent: "21",
    tax_on: today,
  });
  useEffect(() => {
    setPageDirty(true);
    return () => setPageDirty(false);
  }, [setPageDirty]);
  const field = (
    label: AppKey,
    value: string,
    change: (v: string) => void,
    required = false,
  ) => (
    <label className="block space-y-inline">
      <span className="text-sm font-medium">{t(label)}</span>
      <Input
        value={value}
        required={required}
        onChange={(e) => change(e.target.value)}
      />
    </label>
  );
  return (
    <Dialog
      open
      onClose={() => {
        if (!busy) onClose();
      }}
      title={t(form.kind === "account" ? "cashAccount" : "cashManual")}
      description={t(
        form.kind === "account" ? "cashAccountHelp" : "cashManualHelp",
      )}
    >
      <form
        className="space-y-field"
        onSubmit={async (e) => {
          e.preventDefault();
          if (busy) return;
          const r = await execute(
            form.kind === "account"
              ? {
                  request_id: "",
                  action: "account",
                  id: existing?.id ?? null,
                  revision: existing?.revision ?? null,
                  data: {
                    name,
                    currency: "CZK",
                    opening_on: openingOn,
                    opening_balance: opening.replace(",", "."),
                  },
                  archived,
                }
              : {
                  request_id: "",
                  ...(taxEnabled
                    ? { action: "tax_manual" as const, tax }
                    : { action: "manual" as const }),
                  account_id: account,
                  data: {
                    booked_on: date,
                    amount:
                      (direction === "expense" ? "-" : "") +
                      amount.replace(",", "."),
                    currency: "CZK",
                    variable_symbol: vs,
                    counterparty: partner,
                    person,
                    description,
                  },
                },
          );
          if (r) onSaved();
        }}
      >
        {error && <Alert tone="danger">{t(error)}</Alert>}
        <fieldset disabled={busy} className="space-y-field min-w-0">
          {form.kind === "account" ? (
            <>
              {field("cashName", name, setName, true)}
              <DatePicker
                required
                label={t("cashOpeningDate")}
                value={openingOn}
                onChange={setOpeningOn}
              />
              {field("cashOpening", opening, setOpening, true)}
              <p className="text-sm text-muted">{t("cashCurrency")}</p>
              {existing && (
                <Select
                  label={t("cashAvailability")}
                  value={archived ? "archived" : "active"}
                  onChange={(v) => setArchived(v === "archived")}
                  options={[
                    { value: "active", label: t("cashActive") },
                    { value: "archived", label: t("cashArchived") },
                  ]}
                />
              )}
            </>
          ) : (
            <>
              <Select
                required
                label={t("cashAccount")}
                value={account}
                onChange={setAccount}
                options={accounts
                  .filter((a) => !a.archived)
                  .map((a) => ({ value: a.id, label: a.data.name }))}
              />
              <Select
                label={t("cashDirection")}
                value={direction}
                onChange={setDirection}
                options={[
                  { value: "income", label: t("cashIncome") },
                  { value: "expense", label: t("cashExpense") },
                ]}
              />
              <DatePicker
                required
                label={t("cashDate")}
                value={date}
                onChange={setDate}
              />
              <Select
                label={t("cashTaxKind")}
                value={taxEnabled ? "tax" : "payment"}
                onChange={(v) => setTaxEnabled(v === "tax")}
                options={[
                  { value: "payment", label: t("cashTaxPayment") },
                  ...(company?.settings.tax_mode === "vat_payer"
                    ? [{ value: "tax", label: t("cashTaxStandalone") }]
                    : []),
                ]}
              />
              {field(
                taxEnabled ? "cashTaxPrice" : "cashAmount",
                amount,
                setAmount,
                true,
              )}
              {taxEnabled && (
                <CashTaxFields
                  companyId={companyId}
                  amount={
                    (direction === "expense" ? "-" : "") +
                    amount.replace(",", ".")
                  }
                  value={tax}
                  onChange={setTax}
                />
              )}
              {field("cashVs", vs, setVs)}
              {field("cashPartner", partner, setPartner)}
              {field("cashPerson", person, setPerson)}
              {field(
                "cashDescription",
                description,
                setDescription,
                taxEnabled,
              )}
            </>
          )}
        </fieldset>
        <div className="flex flex-wrap justify-end gap-control">
          <Button
            type="button"
            variant="secondary"
            disabled={busy}
            onClick={onClose}
          >
            {t("cashClose")}
          </Button>
          <Button
            type="submit"
            disabled={busy || (form.kind === "manual" && !account)}
          >
            {t(busy ? "cashSaving" : "cashSave")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
