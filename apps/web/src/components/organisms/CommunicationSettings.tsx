import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Dialog,
  Field,
  Heading,
  Switch,
} from "@open-faktura/ui";
import {
  getCommunicationSettings,
  saveCommunicationSettings,
  SalesFailure,
  type CommunicationSettings as Settings,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
export function CommunicationSettings({
  company,
  editable,
}: {
  company: string;
  editable: boolean;
}) {
  const { t } = useAppTranslation();
  const { onExpired } = useWorkspace();
  const [value, setValue] = useState<Settings | null>(null),
    [attempt, setAttempt] = useState(0),
    [busy, setBusy] = useState(false),
    [confirm, setConfirm] = useState(false),
    [saved, setSaved] = useState(false),
    [error, setError] = useState<AppKey | "">("");
  function report(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else
      setError(
        e instanceof SalesFailure && e.status === 409
          ? "communicationConflict"
          : "requestFailed",
      );
  }
  useEffect(() => {
    const abort = new AbortController();
    setValue(null);
    getCommunicationSettings(apiUrl, company, abort.signal)
      .then((v) => {
        if (!abort.signal.aborted) {
          setValue(v);
          setError("");
        }
      })
      .catch((e) => {
        if (!abort.signal.aborted) report(e);
      });
    return () => abort.abort();
  }, [company, attempt]);
  async function save() {
    if (!value || busy) return;
    setBusy(true);
    setError("");
    try {
      setValue(await saveCommunicationSettings(apiUrl, company, value));
      setSaved(true);
      setConfirm(false);
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  }
  function patch(v: Partial<Settings>) {
    setValue((old) => (old ? { ...old, ...v } : old));
    setSaved(false);
  }
  return (
    <section className="workspace-panel">
      <div className="panel-toolbar">
        <Heading level={2} size="card">
          {t("communicationTitle")}
        </Heading>
      </div>
      <div className="p-field space-y-field">
        {error && (
          <Alert tone="danger" role="alert">
            {t(error)}{" "}
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setAttempt((v) => v + 1)}
            >
              {t("paymentRefresh")}
            </Button>
          </Alert>
        )}
        {saved && (
          <Alert tone="success" role="status">
            {t("companySaved")}
          </Alert>
        )}
        {!value && !error && <p role="status">{t("draftLoading")}</p>}
        {value && (
          <form
            className="space-y-field"
            onSubmit={(e) => {
              e.preventDefault();
              if (value.reminders || value.thanks) setConfirm(true);
              else void save();
            }}
          >
            <p className="text-sm text-muted">
              {t("communicationSettingsHelp")}
            </p>
            <Switch
              label={t("communicationAutoReminders")}
              checked={value.reminders}
              disabled={!editable || busy}
              onChange={(e) => patch({ reminders: e.target.checked })}
            />
            <div className="grid gap-field md:grid-cols-3">
              <Field
                label={t("communicationFirstDays")}
                type="number"
                min={1}
                max={365}
                required
                disabled={!editable || busy}
                value={String(value.first_days)}
                onChange={(e) => patch({ first_days: Number(e.target.value) })}
              />
              <Field
                label={t("communicationRepeatDays")}
                type="number"
                min={1}
                max={365}
                required
                disabled={!editable || busy}
                value={String(value.repeat_days)}
                onChange={(e) => patch({ repeat_days: Number(e.target.value) })}
              />
              <Field
                label={t("communicationMinimum")}
                inputMode="decimal"
                required
                disabled={!editable || busy}
                value={value.minimum_total}
                onChange={(e) =>
                  patch({ minimum_total: e.target.value.replace(",", ".") })
                }
              />
            </div>
            <Switch
              label={t("communicationAutoThanks")}
              checked={value.thanks}
              disabled={!editable || busy}
              onChange={(e) => patch({ thanks: e.target.checked })}
            />
            <p className="text-sm text-muted">{t("communicationThanksHelp")}</p>
            {editable && (
              <Button type="submit" loading={busy} disabled={busy}>
                {t("communicationSave")}
              </Button>
            )}
          </form>
        )}
      </div>
      <Dialog
        open={confirm}
        title={t("communicationConfirmTitle")}
        description={t("communicationConfirmHelp")}
        onClose={() => {
          if (!busy) setConfirm(false);
        }}
      >
        {error && (
          <Alert tone="danger" role="alert">
            {t(error)}
          </Alert>
        )}
        <div className="flex justify-end gap-control">
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => setConfirm(false)}
          >
            {t("cancelAction")}
          </Button>
          <Button loading={busy} disabled={busy} onClick={() => void save()}>
            {t("communicationConfirm")}
          </Button>
        </div>
      </Dialog>
    </section>
  );
}
