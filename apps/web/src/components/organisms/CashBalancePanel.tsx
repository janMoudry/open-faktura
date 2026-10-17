import { useEffect, useState } from "react";
import { Alert, Button, DatePicker, Heading, Select } from "@open-faktura/ui";
import {
  cashBalance,
  SalesFailure,
  type CashAccount,
  type CashBalance,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function CashBalancePanel({
  companyId,
  accounts,
  version,
}: {
  companyId: string;
  accounts: CashAccount[];
  version: number;
}) {
  const { t, language } = useAppTranslation(),
    { onExpired } = useWorkspace();
  const [account, setAccount] = useState(""),
    [from, setFrom] = useState(""),
    [to, setTo] = useState(""),
    [value, setValue] = useState<CashBalance | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0);
  const selected = account || accounts[0]?.id || "";
  useEffect(() => {
    setValue(null);
    setError(false);
    if (!selected) return;
    const controller = new AbortController();
    cashBalance(
      apiUrl,
      companyId,
      {
        account_id: selected,
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
  }, [companyId, selected, from, to, version, attempt]);
  if (!accounts.length) return null;
  return (
    <section
      className="workspace-panel p-field space-y-field"
      aria-label={t("cashBalance")}
    >
      <Heading level={2} size="section">
        {t("cashBalance")}
      </Heading>
      <div className="flex flex-wrap items-end gap-field">
        <Select
          label={t("cashAccount")}
          value={selected}
          onChange={setAccount}
          options={accounts.map((a) => ({ value: a.id, label: a.data.name }))}
        />
        <DatePicker label={t("cashFrom")} value={from} onChange={setFrom} />
        <DatePicker label={t("cashTo")} value={to} onChange={setTo} />
      </div>
      <p className="text-sm text-muted">{t("cashBalanceHelp")}</p>
      {error ? (
        <Alert tone="danger">
          {t("cashInvalid")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((v) => v + 1)}>
            {t("cashRetry")}
          </Button>
        </Alert>
      ) : !value ? (
        <p role="status">{t("cashLoading")}</p>
      ) : (
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
                {moneyText(v, language)}
              </dd>
            </div>
          ))}
        </dl>
      )}
    </section>
  );
}
