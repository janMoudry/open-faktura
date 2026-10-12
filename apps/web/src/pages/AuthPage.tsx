import { authMessage } from "../utils/auth-error";
import { useAppTranslation, type AppKey } from "../locales";
import { useState } from "react";
import { Alert, Button, Field, Heading } from "@open-faktura/ui";
import { bootstrap, login } from "@open-faktura/api-client";
import { apiUrl } from "../services/status";

export function AuthPage({
  setup,
  onDone,
}: {
  setup: boolean;
  onDone: () => void;
}) {
  const { t } = useAppTranslation();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [secret, setSecret] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const [created, setCreated] = useState(false);
  const configuring = setup && !created;
  return (
    <>
      <Heading level={1} size="page">
        {configuring ? t("setupTitle") : t("loginTitle")}
      </Heading>
      <p className="mt-field text-sm leading-relaxed text-muted">
        {configuring ? t("setupIntro") : t("loginIntro")}
      </p>
      {created && (
        <Alert tone="success" role="status" className="mt-group">
          {t("created")}
        </Alert>
      )}
      <form
        noValidate
        className="mt-group space-y-field"
        onSubmit={async (e) => {
          e.preventDefault();
          if (busy) return;
          if (!e.currentTarget.checkValidity()) {
            setError("invalidForm");
            e.currentTarget
              .querySelector<HTMLInputElement>(":invalid")
              ?.focus();
            return;
          }
          if (configuring && password !== confirmation) {
            setError("passwordMismatch");
            return;
          }
          setBusy(true);
          setError("");
          try {
            const signal = AbortSignal.timeout(15000);
            if (configuring) {
              await bootstrap(apiUrl, { email, password, secret }, signal);
              setCreated(true);
              setSecret("");
              setPassword("");
              setConfirmation("");
            } else {
              await login(apiUrl, { email, password }, signal);
              onDone();
            }
          } catch (error) {
            setError(authMessage(error));
          } finally {
            setBusy(false);
          }
        }}
      >
        <Field
          label={t("email")}
          type="email"
          autoComplete="username"
          required
          maxLength={254}
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          disabled={busy}
        />
        <Field
          label={t("password")}
          type="password"
          autoComplete={configuring ? "new-password" : "current-password"}
          required
          minLength={configuring ? 12 : undefined}
          maxLength={1024}
          hint={configuring ? t("passwordHint") : undefined}
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={busy}
        />
        {configuring && (
          <>
            <Field
              label={t("confirmPassword")}
              type="password"
              autoComplete="new-password"
              required
              value={confirmation}
              onChange={(e) => setConfirmation(e.target.value)}
              disabled={busy}
            />
            <Field
              label={t("installationKey")}
              help={
                <div className="space-y-control">
                  <p className="font-semibold">{t("keyPurpose")}</p>
                  <p>{t("keyExplanation")}</p>
                  <div>
                    <p className="font-semibold">{t("keyLocation")}</p>
                    <p className="mt-inline">
                      {t("keyFile")}
                      <br />
                      <code className="rounded-control bg-canvas px-1">
                        .secrets/bootstrap
                      </code>
                    </p>
                    <p className="mt-inline">{t("keyOperator")}</p>
                  </div>
                  <p className="text-xs text-muted">{t("keySafety")}</p>
                </div>
              }
              type="password"
              autoComplete="off"
              required
              minLength={32}
              maxLength={512}
              value={secret}
              onChange={(e) => setSecret(e.target.value)}
              disabled={busy}
            />
          </>
        )}
        {error && (
          <Alert tone="danger" role="alert">
            {t(error)}
          </Alert>
        )}
        <Button type="submit" loading={busy}>
          {configuring ? t("createAdmin") : t("signIn")}
        </Button>
        {!configuring && (
          <p className="text-xs leading-relaxed text-muted">
            {t("forgotPassword")}
          </p>
        )}
      </form>
      <a
        className="mt-group inline-block text-sm underline underline-offset-4"
        href={configuring ? "#/login" : "#/setup"}
      >
        {configuring ? t("existingAdmin") : t("checkSetup")}
      </a>
    </>
  );
}
