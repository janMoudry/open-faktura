import { useEffect, useState } from "react";
import { Alert, Button, Field, Heading, Select } from "@open-faktura/ui";
import {
  inboxSettings,
  inboxConfigure,
  inboxMailHistory,
  type InboxSettings,
  type InboxMailDiagnostic,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";

export function InboxAdapters({
  companyId,
  write,
}: {
  companyId: string;
  write: boolean;
}) {
  const { t, language } = useAppTranslation();
  const [settings, setSettings] = useState<InboxSettings | null>(null);
  const [history, setHistory] = useState<InboxMailDiagnostic[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    let active = true;
    setSettings(null);
    setHistory([]);
    setError(false);
    setSaved(false);
    Promise.all([
      inboxSettings(apiUrl, companyId),
      inboxMailHistory(apiUrl, companyId),
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
    try {
      setSettings(await inboxConfigure(apiUrl, companyId, settings));
      setSaved(true);
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="workspace-panel space-y-field p-field">
      <Heading level={2}>{t("inboxAdapters")}</Heading>
      <p className="text-muted">{t("inboxAdaptersDescription")}</p>
      {error && <Alert tone="danger">{t("inboxFailed")}</Alert>}
      {saved && <p role="status">{t("inboxAdapterSaved")}</p>}
      {settings && (
        <div className="flex flex-wrap items-end gap-field">
          <Select
            label={t("inboxEmailEnabled")}
            disabled={!write || busy}
            value={String(settings.email_enabled)}
            onChange={(value) => {
              setSettings({ ...settings, email_enabled: value === "true" });
              setSaved(false);
            }}
            options={[
              { value: "false", label: t("inboxAdapterDisabled") },
              { value: "true", label: t("inboxAdapterEnabled") },
            ]}
          />
          <Field
            label={t("inboxExtractionProvider")}
            disabled={!write || busy}
            value={settings.extraction_provider ?? ""}
            maxLength={80}
            onChange={(e) => {
              setSettings({
                ...settings,
                extraction_provider: e.target.value || null,
              });
              setSaved(false);
            }}
          />
          {write && (
            <Button disabled={busy} onClick={save}>
              {t("inboxAdapterSave")}
            </Button>
          )}
        </div>
      )}
      <p className="text-sm text-muted">{t("inboxExtractionPrivacy")}</p>
      <Heading level={3}>{t("inboxMailHistory")}</Heading>
      {!history.length && <p>{t("inboxEmpty")}</p>}
      <ul className="space-y-control">
        {history.map((entry) => (
          <li key={entry.message_id} className="break-words">
            {new Date(entry.created_at).toLocaleString(language)} ·{" "}
            {entry.sender} · {entry.subject} · {entry.attachment_count}{" "}
            {t("inboxMailAttachments")}
          </li>
        ))}
      </ul>
    </section>
  );
}
