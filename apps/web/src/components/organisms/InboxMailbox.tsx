import { useEffect, useState } from "react";
import { Alert, Button, Field, Heading, Select } from "@open-faktura/ui";
import {
  inboxMailboxSettings,
  inboxMailboxConfigure,
  inboxMailboxSync,
  inboxMailboxHistory,
  type MailboxSettings,
  type MailboxRun,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
export function InboxMailbox({
  companyId,
  write,
  onImported,
}: {
  companyId: string;
  write: boolean;
  onImported: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [settings, setSettings] = useState<MailboxSettings | null>(null),
    [history, setHistory] = useState<MailboxRun[]>([]),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(false),
    [saved, setSaved] = useState(false),
    [dirty, setDirty] = useState(false);
  useEffect(() => {
    let active = true;
    setSettings(null);
    setHistory([]);
    setError(false);
    Promise.all([
      inboxMailboxSettings(apiUrl, companyId),
      inboxMailboxHistory(apiUrl, companyId),
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
  }, [companyId]);
  async function save() {
    if (!settings || busy) return;
    setBusy(true);
    setError(false);
    setSaved(false);
    setDirty(true);
    try {
      setSettings(
        await inboxMailboxConfigure(apiUrl, companyId, {
          revision: settings.revision,
          enabled: settings.enabled,
          automatic: settings.automatic,
          provider: settings.provider,
        }),
      );
      setSaved(true);
      setDirty(false);
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  async function sync() {
    if (busy) return;
    setBusy(true);
    setError(false);
    try {
      await inboxMailboxSync(apiUrl, companyId, crypto.randomUUID());
      setSettings(await inboxMailboxSettings(apiUrl, companyId));
      setHistory(await inboxMailboxHistory(apiUrl, companyId));
      onImported();
    } catch {
      setError(true);
      setHistory(await inboxMailboxHistory(apiUrl, companyId).catch(() => []));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="workspace-panel p-field space-y-field">
      <Heading level={2}>{t("mailboxTitle")}</Heading>
      <p className="text-muted">{t("mailboxDescription")}</p>
      {error && (
        <Alert tone="danger" role="alert">
          {t("mailboxFailed")}
        </Alert>
      )}
      {saved && <p role="status">{t("inboxAdapterSaved")}</p>}
      {busy && <p role="status">{t("mailboxBusy")}</p>}
      {settings && (
        <>
          <div className="flex flex-wrap items-end gap-field">
            <Field
              label={t("mailboxProvider")}
              maxLength={80}
              value={settings.provider ?? ""}
              disabled={!write || busy}
              onChange={(e) => {
                setSettings({ ...settings, provider: e.target.value || null });
                setSaved(false);
                setDirty(true);
              }}
            />
            <Select
              label={t("mailboxEnabled")}
              value={String(settings.enabled)}
              disabled={!write || busy}
              onChange={(v) => {
                setSettings({
                  ...settings,
                  enabled: v === "true",
                  automatic: v === "true" && settings.automatic,
                });
                setSaved(false);
                setDirty(true);
              }}
              options={[
                { value: "false", label: t("inboxAdapterDisabled") },
                { value: "true", label: t("inboxAdapterEnabled") },
              ]}
            />
            <Select
              label={t("mailboxAutomatic")}
              value={String(settings.automatic)}
              disabled={!write || busy || !settings.enabled}
              onChange={(v) => {
                setSettings({ ...settings, automatic: v === "true" });
                setDirty(true);
                setSaved(false);
              }}
              options={[
                { value: "false", label: t("inboxAdapterDisabled") },
                { value: "true", label: t("inboxAdapterEnabled") },
              ]}
            />
            {write && (
              <>
                <Button disabled={busy} onClick={save}>
                  {t("inboxAdapterSave")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={
                    busy || dirty || !settings.enabled || !settings.provider
                  }
                  onClick={sync}
                >
                  {t("mailboxSync")}
                </Button>
              </>
            )}
          </div>
          <p className="text-sm text-muted">{t("mailboxScheduleHint")}</p>
          <p className="text-sm text-muted">
            {t("mailboxCursor", {
              cursor: settings.cursor,
              validity: settings.uid_validity ?? "—",
            })}
          </p>
        </>
      )}
      <Heading level={3}>{t("mailboxHistory")}</Heading>
      {!history.length && <p>{t("inboxEmpty")}</p>}
      <ul className="space-y-control">
        {history.map((run) => (
          <li key={run.request_id} className="break-words">
            {new Date(run.created_at).toLocaleString(language)} ·{" "}
            {t(
              run.status === "completed"
                ? "mailboxCompleted"
                : run.status === "running"
                  ? "mailboxRunning"
                  : "mailboxError",
            )}{" "}
            · {run.attachments} {t("inboxMailAttachments")} · {run.rejected}{" "}
            {t("mailboxRejected")}
          </li>
        ))}
      </ul>
    </section>
  );
}
