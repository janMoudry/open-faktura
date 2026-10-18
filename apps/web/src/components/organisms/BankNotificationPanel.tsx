import { useEffect, useState } from "react";
import { Alert, Button, Heading, Select } from "@open-faktura/ui";
import {
  bankNotificationSettings,
  bankNotificationConfigure,
  bankNotificationHistory,
  type BankNotificationSettings,
  type BankNotificationDiagnostic,
  type BankAccount,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
export function BankNotificationPanel({
  companyId,
  accounts,
  write,
}: {
  companyId: string;
  accounts: BankAccount[];
  write: boolean;
}) {
  const { t } = useAppTranslation();
  const [selected, setSelected] = useState("");
  const account = accounts.find((a) => a.id === selected) ?? accounts[0];
  return (
    <section className="workspace-panel space-y-field p-field">
      <Heading level={2}>{t("bankNotificationTitle")}</Heading>
      <p className="text-muted">{t("bankNotificationDescription")}</p>
      {!!accounts.length && (
        <Select
          label={t("bankNotificationAccount")}
          value={account?.id ?? ""}
          onChange={setSelected}
          options={accounts.map((a) => ({ value: a.id, label: a.data.name }))}
        />
      )}
      {account && (
        <AccountPanel
          key={`${companyId}:${account.id}`}
          companyId={companyId}
          account={account}
          write={write && !account.archived}
        />
      )}
    </section>
  );
}
function AccountPanel({
  companyId,
  account,
  write,
}: {
  companyId: string;
  account: BankAccount;
  write: boolean;
}) {
  const { t, language } = useAppTranslation();
  const [settings, setSettings] = useState<BankNotificationSettings | null>(
      null,
    ),
    [history, setHistory] = useState<BankNotificationDiagnostic[]>([]),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(false),
    [saved, setSaved] = useState(false);
  useEffect(() => {
    let active = true;
    Promise.all([
      bankNotificationSettings(apiUrl, companyId, account.id),
      bankNotificationHistory(apiUrl, companyId, account.id),
    ])
      .then(([s, h]) => {
        if (active) {
          setSettings(s);
          setHistory(h);
        }
      })
      .catch(() => {
        if (active) setError(true);
      });
    return () => {
      active = false;
    };
  }, [companyId, account.id]);
  async function save() {
    if (!settings || busy) return;
    setBusy(true);
    setError(false);
    setSaved(false);
    try {
      setSettings(await bankNotificationConfigure(apiUrl, companyId, settings));
      setSaved(true);
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="space-y-field">
      {error && <Alert tone="danger">{t("bankNotificationFailed")}</Alert>}
      {saved && <p role="status">{t("inboxAdapterSaved")}</p>}
      {settings && (
        <div className="flex flex-wrap items-end gap-field">
          <Select
            label={t("bankNotificationEnabled")}
            value={String(settings.enabled)}
            disabled={!write || busy}
            onChange={(v) => {
              setSettings({ ...settings, enabled: v === "true" });
              setSaved(false);
            }}
            options={[
              { value: "false", label: t("inboxAdapterDisabled") },
              { value: "true", label: t("inboxAdapterEnabled") },
            ]}
          />
          <Select
            label={t("bankNotificationFormat")}
            value={settings.format}
            disabled={!write || busy}
            onChange={(v) => {
              setSettings({
                ...settings,
                format: v as BankNotificationSettings["format"],
              });
              setSaved(false);
            }}
            options={[
              { value: "camt053", label: "ISO 20022 CAMT.053" },
              { value: "fio_json", label: "Fio JSON" },
              { value: "moneta_xml", label: "MONETA XML" },
              { value: "rb_abo", label: "Raiffeisenbank ABO" },
            ]}
          />
          {write && (
            <Button disabled={busy} onClick={save}>
              {t("inboxAdapterSave")}
            </Button>
          )}
        </div>
      )}
      <Heading level={3}>{t("bankNotificationHistory")}</Heading>
      {!history.length && <p>{t("inboxEmpty")}</p>}
      <ul className="space-y-control">
        {history.map((h) => (
          <li key={h.message_id} className="break-words">
            {new Date(h.created_at).toLocaleString(language)} · {h.sender} ·{" "}
            {h.new_count} {t("bankNotificationNew")} · {h.duplicate_count}{" "}
            {t("bankNotificationDuplicates")}
          </li>
        ))}
      </ul>
    </div>
  );
}
