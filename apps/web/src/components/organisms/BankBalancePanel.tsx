import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  DatePicker,
  Field,
  Heading,
  Select,
} from "@open-faktura/ui";
import {
  bankBalance,
  SalesFailure,
  type BankAccount,
  type BankBalance,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { useBankCommand } from "../../hooks/useBankCommand";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function BankBalancePanel({
  companyId,
  accounts,
  version,
  onSaved,
}: {
  companyId: string;
  accounts: BankAccount[];
  version: number;
  onSaved: () => void;
}) {
  const { t } = useAppTranslation();
  const [selected, setSelected] = useState("");
  const account = accounts.find((a) => a.id === selected) ?? accounts[0];
  if (!account) return null;
  return (
    <section
      className="workspace-panel p-field space-y-field"
      aria-label={t("bankBalance")}
    >
      <Heading level={2} size="section">
        {t("bankBalance")}
      </Heading>
      <Select
        label={t("bankAccount")}
        value={account.id}
        onChange={setSelected}
        options={accounts.map((a) => ({ value: a.id, label: a.data.name }))}
      />
      <Balance
        key={`${companyId}:${account.id}:${version}`}
        companyId={companyId}
        account={account}
        onSaved={onSaved}
      />
    </section>
  );
}
function Balance({
  companyId,
  account,
  onSaved,
}: {
  companyId: string;
  account: BankAccount;
  onSaved: () => void;
}) {
  const { t, language } = useAppTranslation(),
    { company, onExpired } = useWorkspace(),
    cmd = useBankCommand(companyId);
  const [from, setFrom] = useState(""),
    [to, setTo] = useState(""),
    [value, setValue] = useState<BankBalance | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0),
    [editing, setEditing] = useState(false),
    [date, setDate] = useState(""),
    [amount, setAmount] = useState(""),
    [reason, setReason] = useState("");
  useEffect(() => {
    setValue(null);
    setError(false);
    const controller = new AbortController();
    void bankBalance(
      apiUrl,
      companyId,
      {
        account_id: account.id,
        ...(from ? { from } : {}),
        ...(to ? { to } : {}),
      },
      controller.signal,
    )
      .then((v) => {
        if (!controller.signal.aborted) setValue(v);
      })
      .catch((e) => {
        if (!controller.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => controller.abort();
  }, [companyId, account.id, from, to, attempt, onExpired]);
  return (
    <div className="space-y-field">
      <p className="text-sm text-muted">{t("bankBalanceHelp")}</p>
      <div className="flex flex-wrap gap-field">
        <DatePicker label={t("cashFrom")} value={from} onChange={setFrom} />
        <DatePicker label={t("cashTo")} value={to} onChange={setTo} />
      </div>
      {error ? (
        <Alert tone="danger">
          {t("bankBalanceError")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((x) => x + 1)}>
            {t("bankRetry")}
          </Button>
        </Alert>
      ) : !value ? (
        <p role="status">{t("bankLoading")}</p>
      ) : (
        <>
          {value.opening === null && <Alert>{t("bankOpeningMissing")}</Alert>}
          <dl className="grid grid-cols-2 lg:grid-cols-4 gap-field">
            {(
              [
                ["cashOpening", value.opening],
                ["cashIncome", value.income],
                ["cashExpense", value.expense],
                ["cashClosing", value.closing],
              ] as const
            ).map(([label, v]) => (
              <div key={label}>
                <dt className="text-sm text-muted">{t(label)}</dt>
                <dd className="font-semibold tabular-nums">
                  {v === null ? "—" : moneyText(v, language)}
                </dd>
              </div>
            ))}
          </dl>
          {company?.role !== "viewer" && !account.archived && !editing && (
            <Button
              variant="secondary"
              onClick={() => {
                setDate(
                  value.opening_on ??
                    new Intl.DateTimeFormat("en-CA", {
                      timeZone: "Europe/Prague",
                      year: "numeric",
                      month: "2-digit",
                      day: "2-digit",
                    }).format(new Date()),
                );
                setAmount(value.opening_balance ?? "0.00");
                setEditing(true);
              }}
            >
              {t("bankOpeningEdit")}
            </Button>
          )}
        </>
      )}
      {editing && (
        <form
          className="space-y-field"
          onSubmit={async (e) => {
            e.preventDefault();
            if (cmd.busy) return;
            const r = await cmd.execute({
              action: "opening",
              request_id: "",
              account_id: account.id,
              revision: account.revision,
              opening_on: date,
              opening_balance: amount.replace(",", "."),
              reason,
            });
            if (r) {
              setEditing(false);
              onSaved();
            }
          }}
        >
          <Alert>{t("bankOpeningHelp")}</Alert>
          {cmd.error && <Alert tone="danger">{t(cmd.error)}</Alert>}
          <fieldset
            disabled={cmd.busy}
            className="grid gap-field sm:grid-cols-2"
          >
            <DatePicker
              required
              label={t("cashOpeningDate")}
              value={date}
              onChange={setDate}
            />
            <Field
              required
              label={t("cashOpening")}
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
            <Field
              required
              label={t("bankReason")}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
            />
          </fieldset>
          <div className="flex gap-control">
            <Button type="submit" disabled={cmd.busy}>
              {t(cmd.busy ? "bankSaving" : "bankSave")}
            </Button>
            <Button
              type="button"
              variant="secondary"
              disabled={cmd.busy}
              onClick={() => setEditing(false)}
            >
              {t("bankClose")}
            </Button>
          </div>
        </form>
      )}
    </div>
  );
}
