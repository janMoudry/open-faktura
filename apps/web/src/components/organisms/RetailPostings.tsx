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
  cashAccounts,
  retailPostings,
  retailPostingPreview,
  retailPost,
  retailUnpost,
  type CashAccount,
  type RetailPosting,
  type RetailPostingPreview,
  type RetailPostingInput,
  SalesFailure,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
export function RetailPostings({
  companyId,
  selected,
  canWrite,
  onChanged,
}: {
  companyId: string;
  selected: string[];
  canWrite: boolean;
  onChanged: () => void;
}) {
  const { t, language } = useAppTranslation(),
    { onExpired } = useWorkspace();
  const [items, setItems] = useState<RetailPosting[]>([]),
    [accounts, setAccounts] = useState<CashAccount[]>([]),
    [date, setDate] = useState(new Date().toLocaleDateString("sv-SE")),
    [account, setAccount] = useState(""),
    [preview, setPreview] = useState<RetailPostingPreview | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(false),
    [reason, setReason] = useState(""),
    [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const abort = new AbortController();
    setError(false);
    Promise.all([
      retailPostings(apiUrl, companyId, abort.signal),
      cashAccounts(apiUrl, companyId, abort.signal),
    ])
      .then(([p, a]) => {
        setItems(p);
        setAccounts(a.filter((v) => !v.archived));
      })
      .catch((e) => {
        if (!abort.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => abort.abort();
  }, [companyId, attempt]);
  const selection = selected.join(",");
  useEffect(() => setPreview(null), [selection, date, account]);
  const input: RetailPostingInput = {
    document_ids: selected,
    booked_on: date,
    cash_account_id: account || null,
  };
  async function run(action: () => Promise<unknown>, changed = false) {
    setBusy(true);
    setError(false);
    try {
      await action();
      if (changed) {
        setPreview(null);
        setAttempt((v) => v + 1);
        onChanged();
      }
    } catch (e) {
      setError(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="workspace-panel min-w-0 space-y-field p-field">
      <Heading level={2} size="section">
        {t("retailPostingTitle")}
      </Heading>
      <p className="text-sm text-muted">{t("retailPostingHelp")}</p>
      {error && (
        <Alert tone="danger">
          {t("retailPostingFailed")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((v) => v + 1)}>
            {t("cashRetry")}
          </Button>
        </Alert>
      )}
      {canWrite && (
        <div className="flex flex-wrap items-end gap-field">
          <DatePicker
            label={t("retailPostingDate")}
            value={date}
            onChange={setDate}
            disabled={busy}
          />
          <Select
            label={t("retailPostingAccount")}
            value={account}
            onChange={setAccount}
            disabled={busy}
            options={[
              { value: "", label: "—" },
              ...accounts.map((a) => ({ value: a.id, label: a.data.name })),
            ]}
          />
          <Button
            variant="secondary"
            disabled={busy || !selected.length}
            onClick={() =>
              run(async () =>
                setPreview(
                  await retailPostingPreview(apiUrl, companyId, input),
                ),
              )
            }
          >
            {t("retailPostingPreview")} ({selected.length})
          </Button>
        </div>
      )}
      {preview && (
        <div className="space-y-field">
          {preview.groups.map((g, i) => (
            <p key={i}>
              {g.terminal?.name ?? t("retailNoTerminal")} ·{" "}
              {g.customer?.name ?? t("retailAnonymous")} ·{" "}
              {t(
                g.method === "cash"
                  ? "retailCash"
                  : g.method === "card"
                    ? "retailCard"
                    : "retailVoucher",
              )}{" "}
              — {moneyText(g.paid, language)}
            </p>
          ))}
          <Button
            disabled={busy}
            onClick={() =>
              run(
                () =>
                  retailPost(apiUrl, companyId, {
                    request_id: crypto.randomUUID(),
                    input,
                    fingerprint: preview.fingerprint,
                  }),
                true,
              )
            }
          >
            {t("retailPostingConfirm")}
          </Button>
        </div>
      )}
      <div className="space-y-field">
        {items.map((v) => (
          <div
            key={v.id}
            className="space-y-control border-t border-border pt-field"
          >
            <p>
              {new Date(v.booked_on + "T12:00:00").toLocaleDateString(language)}{" "}
              · {t(v.cancelled ? "retailCancelled" : "retailPosted")} · #{v.id}
            </p>
            {v.groups.map((g, i) => (
              <p className="text-sm text-muted" key={i}>
                {t(
                  g.method === "cash"
                    ? "retailCash"
                    : g.method === "card"
                      ? "retailCard"
                      : "retailVoucher",
                )}{" "}
                · {moneyText(g.paid, language)} ·{" "}
                {g.internal_number ?? g.cash_movement_id}
              </p>
            ))}
            {canWrite && !v.cancelled && (
              <Button
                variant="secondary"
                disabled={busy || !reason.trim()}
                onClick={() =>
                  run(
                    () =>
                      retailUnpost(apiUrl, companyId, {
                        request_id: crypto.randomUUID(),
                        id: v.id,
                        revision: v.revision,
                        reason,
                      }),
                    true,
                  )
                }
              >
                {t("retailPostingReverse")}
              </Button>
            )}
          </div>
        ))}
        {canWrite && items.some((v) => !v.cancelled) && (
          <Field
            label={t("retailReason")}
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            maxLength={500}
          />
        )}
      </div>
    </section>
  );
}
