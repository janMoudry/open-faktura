import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Button,
  Dialog,
  Field,
  Heading,
  Select,
} from "@open-faktura/ui";
import {
  getSmtp,
  saveSmtp,
  testSmtp,
  SalesFailure,
  type SmtpState,
  type SmtpSave,
  type SmtpTestResult,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
function form(s: SmtpState): SmtpSave {
  return {
    revision: s.revision,
    mode: s.mode,
    host: s.host,
    port: s.port,
    security: s.security,
    sender: s.sender,
    username: s.username,
    password: null,
  };
}
export function SmtpSettingsDialog({
  email,
  onClose,
  onExpired,
}: {
  email: string;
  onClose: () => void;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [reload, setReload] = useState(0);
  const [saved, setSaved] = useState<SmtpState>();
  const [draft, setDraft] = useState<SmtpSave>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const [notice, setNotice] = useState(false);
  const [recipient, setRecipient] = useState(email);
  const [result, setResult] = useState<SmtpTestResult>();
  const attempt = useRef<
    | {
        request_id: string;
        revision: number;
        recipient: string;
        language: "cs" | "en";
      }
    | undefined
  >(undefined);
  const dirty =
    !!saved && JSON.stringify(draft) !== JSON.stringify(form(saved));
  function failure(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) {
      onExpired();
      return;
    }
    setError(
      e instanceof SalesFailure && e.status === 409
        ? "smtpConflict"
        : e instanceof SalesFailure && e.status === 403
          ? "smtpForbidden"
          : e instanceof SalesFailure && e.fields.includes("rate_limit")
            ? "smtpRateLimit"
            : "smtpError",
    );
  }
  useEffect(() => {
    let active = true;
    getSmtp(apiUrl)
      .then((s) => {
        if (active) {
          setSaved(s);
          setDraft(form(s));
        }
      })
      .catch((e) => {
        if (active) failure(e);
      });
    return () => {
      active = false;
    };
  }, [reload]);
  const close = () => {
    if (!busy && (!dirty || window.confirm(t("smtpDiscard")))) onClose();
  };
  const update = <K extends keyof SmtpSave>(k: K, v: SmtpSave[K]) => {
    setDraft((s) => (s ? { ...s, [k]: v } : s));
    setNotice(false);
    setResult(undefined);
  };
  return (
    <Dialog
      open
      title={t("smtpTitle")}
      description={t("smtpDescription")}
      onClose={close}
    >
      {error && (
        <Alert tone="danger" role="alert">
          {t(error)}
        </Alert>
      )}
      {!draft ? (
        error ? (
          <Button
            variant="secondary"
            onClick={() => {
              setError("");
              setReload((v) => v + 1);
            }}
          >
            {t("smtpReload")}
          </Button>
        ) : (
          <p role="status">{t("smtpLoading")}</p>
        )
      ) : (
        <div className="space-y-group">
          <form
            className="space-y-field"
            onSubmit={async (e) => {
              e.preventDefault();
              setBusy(true);
              setError("");
              setNotice(false);
              try {
                const s = await saveSmtp(apiUrl, draft);
                setSaved(s);
                setDraft(form(s));
                setNotice(true);
                window.dispatchEvent(new Event("smtp-settings-changed"));
                setResult(undefined);
                attempt.current = undefined;
              } catch (e) {
                failure(e);
              } finally {
                setBusy(false);
              }
            }}
          >
            <fieldset disabled={busy} className="space-y-field">
              <Select
                disabled={busy}
                label={t("smtpMode")}
                value={draft.mode}
                onChange={(v) => update("mode", v as SmtpSave["mode"])}
                options={[
                  { value: "environment", label: t("smtpEnvironment") },
                  { value: "custom", label: t("smtpCustom") },
                  { value: "disabled", label: t("smtpDisabled") },
                ]}
              />
              {draft.mode === "environment" && (
                <p className="text-sm text-muted">{t("smtpEnvironmentHint")}</p>
              )}
              {draft.mode === "disabled" && (
                <Alert tone="warning">{t("smtpDisabledHint")}</Alert>
              )}
              {draft.mode === "custom" && (
                <>
                  <div className="grid gap-field sm:grid-cols-2">
                    <Field
                      label={t("smtpHost")}
                      required
                      value={draft.host}
                      onChange={(e) => update("host", e.target.value)}
                      autoComplete="off"
                    />
                    <Field
                      label={t("smtpPort")}
                      required
                      type="number"
                      min={1}
                      max={65535}
                      value={draft.port}
                      onChange={(e) => update("port", Number(e.target.value))}
                    />
                  </div>

                  <Select
                    disabled={busy}
                    label={t("smtpSecurity")}
                    value={draft.security}
                    onChange={(v) => {
                      update("security", v as SmtpSave["security"]);
                      update("port", v === "tls" ? 465 : 587);
                    }}
                    options={[
                      { value: "starttls", label: "STARTTLS" },
                      { value: "tls", label: "TLS" },
                      { value: "plain", label: t("smtpPlain") },
                    ]}
                  />
                  <Field
                    label={t("smtpSender")}
                    type="email"
                    required
                    value={draft.sender}
                    onChange={(e) => update("sender", e.target.value)}
                  />
                  <Field
                    label={t("smtpUsername")}
                    value={draft.username}
                    autoComplete="off"
                    onChange={(e) => update("username", e.target.value)}
                  />
                  <Field
                    label={t("smtpPassword")}
                    type="password"
                    autoComplete="new-password"
                    value={draft.password ?? ""}
                    onChange={(e) => update("password", e.target.value || null)}
                    hint={
                      saved?.password_set
                        ? t("smtpPasswordSaved")
                        : t("smtpPasswordHint")
                    }
                  />
                  {!saved?.encryption_ready && (
                    <Alert tone="warning">{t("smtpKeyMissing")}</Alert>
                  )}
                </>
              )}
              <div className="flex flex-wrap justify-end gap-control">
                <Button type="button" variant="secondary" onClick={close}>
                  {t("smtpClose")}
                </Button>
                <Button type="submit" disabled={!dirty || busy}>
                  {t(busy ? "smtpWorking" : "smtpSave")}
                </Button>
              </div>
            </fieldset>
            {notice && (
              <p role="status" className="text-sm text-success">
                {t("smtpSaved")}
              </p>
            )}
          </form>
          <form
            className="space-y-field border-t border-border pt-group"
            onSubmit={async (e) => {
              e.preventDefault();
              if (!saved) return;
              setBusy(true);
              setError("");
              setResult(undefined);
              const old = attempt.current;
              if (
                !old ||
                old.recipient !== recipient ||
                old.revision !== saved.revision
              )
                attempt.current = {
                  request_id: crypto.randomUUID(),
                  revision: saved.revision,
                  recipient,
                  language,
                };
              try {
                const r = await testSmtp(apiUrl, attempt.current!);
                setResult(r);
                if (r.status !== "sending") attempt.current = undefined;
              } catch (e) {
                failure(e);
              } finally {
                setBusy(false);
              }
            }}
          >
            <Heading level={3} size="card">
              {t("smtpTestTitle")}
            </Heading>
            <p className="text-sm text-muted">{t("smtpTestHint")}</p>
            <Field
              label={t("smtpTestRecipient")}
              type="email"
              required
              value={recipient}
              disabled={busy}
              onChange={(e) => {
                setRecipient(e.target.value);
                setResult(undefined);
              }}
            />
            {dirty && (
              <p className="text-sm text-muted">{t("smtpSaveFirst")}</p>
            )}
            <div className="flex justify-end">
              <Button
                type="submit"
                variant="secondary"
                disabled={busy || dirty || !saved?.configured}
              >
                {t(busy ? "smtpWorking" : "smtpTestSend")}
              </Button>
            </div>
            {result && (
              <Alert
                role="status"
                tone={
                  result.status === "sent"
                    ? "success"
                    : result.status === "failed"
                      ? "danger"
                      : "warning"
                }
              >
                {t(
                  result.status === "sent"
                    ? "smtpTestSent"
                    : result.status === "failed"
                      ? "smtpTestFailed"
                      : result.status === "sending"
                        ? "smtpTestSending"
                        : "smtpTestUncertain",
                )}
              </Alert>
            )}
          </form>
        </div>
      )}
    </Dialog>
  );
}
