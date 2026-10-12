import { useState } from "react";
import { Alert, Button, Dialog, Field, Icon } from "@open-faktura/ui";
import {
  AuthFailure,
  updateAccount,
  type AuthUser,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
import { authMessage } from "../../utils/auth-error";

export function AccountSettingsDialog({
  user,
  onClose,
  onSaved,
}: {
  user: AuthUser;
  onClose: () => void;
  onSaved: (user: AuthUser) => void;
}) {
  const { t } = useAppTranslation();
  const [name, setName] = useState(user.name);
  const [email, setEmail] = useState(user.email);
  const [current, setCurrent] = useState("");
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [changingPassword, setChangingPassword] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const [saved, setSaved] = useState(false);
  const sensitive = email.trim().toLowerCase() !== user.email || !!password;
  const dirty =
    name !== user.name ||
    email !== user.email ||
    !!password ||
    !!current ||
    !!confirmation;
  const [confirmClose, setConfirmClose] = useState(false);
  return (
    <Dialog
      open
      onClose={() => {
        if (!busy) {
          if (dirty) setConfirmClose(true);
          else onClose();
        }
      }}
      title={t("accountSettings")}
      description={t("accountIntro")}
    >
      <form
        className="space-y-group"
        noValidate
        onChange={() => {
          setSaved(false);
          setError("");
        }}
        onSubmit={async (event) => {
          event.preventDefault();
          if (busy) return;
          if (!event.currentTarget.checkValidity()) {
            setError("invalidForm");
            event.currentTarget
              .querySelector<HTMLInputElement>(":invalid")
              ?.focus();
            return;
          }
          if (password !== confirmation) {
            setError("passwordMismatch");
            return;
          }
          setBusy(true);
          setError("");
          setSaved(false);
          try {
            const updated = await updateAccount(
              apiUrl,
              {
                name,
                email,
                current_password: current,
                new_password: password || null,
              },
              AbortSignal.timeout(15000),
            );
            onSaved(updated);
            setName(updated.name);
            setEmail(updated.email);
            setCurrent("");
            setPassword("");
            setConfirmation("");
            setSaved(true);
            setChangingPassword(false);
            setConfirmClose(false);
          } catch (error) {
            setError(
              error instanceof AuthFailure && error.code === "email_taken"
                ? "accountEmailTaken"
                : error instanceof AuthFailure &&
                    error.code === "invalid_password"
                  ? "accountWrongPassword"
                  : error instanceof AuthFailure && error.code === "conflict"
                    ? "accountConflict"
                    : authMessage(error),
            );
          } finally {
            setBusy(false);
          }
        }}
      >
        <fieldset disabled={busy} className="space-y-field">
          <Field
            label={t("accountName")}
            autoComplete="name"
            maxLength={120}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <Field
            label={t("email")}
            type="email"
            autoComplete="email"
            required
            maxLength={254}
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </fieldset>
        <fieldset
          disabled={busy}
          className="space-y-field border-t border-border pt-group"
        >
          <Button
            variant="ghost"
            className="w-full justify-between"
            aria-expanded={changingPassword}
            onClick={() => {
              setChangingPassword(!changingPassword);
              setPassword("");
              setConfirmation("");
            }}
          >
            {t("accountPasswordTitle")}
            <Icon name={changingPassword ? "chevronDown" : "chevronRight"} />
          </Button>
          {changingPassword && (
            <div className="space-y-field">
              <p className="text-sm text-muted">{t("accountPasswordHint")}</p>
              <Field
                label={t("accountNewPassword")}
                type="password"
                autoComplete="new-password"
                minLength={12}
                maxLength={1024}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
              <Field
                label={t("confirmPassword")}
                type="password"
                autoComplete="new-password"
                required={!!password}
                value={confirmation}
                onChange={(e) => setConfirmation(e.target.value)}
              />
            </div>
          )}
          {sensitive && (
            <Field
              label={t("accountCurrentPassword")}
              hint={t("accountSecurityHint")}
              type="password"
              autoComplete="current-password"
              required
              maxLength={1024}
              value={current}
              onChange={(e) => setCurrent(e.target.value)}
            />
          )}
        </fieldset>
        {error && (
          <Alert tone="danger" role="alert">
            {t(error)}
          </Alert>
        )}
        {saved && (
          <Alert tone="success" role="status">
            {t("accountSaved")}
          </Alert>
        )}
        {confirmClose && (
          <Alert tone="warning" role="alert">
            <p>{t("accountDiscard")}</p>
            <div className="mt-field flex gap-inline">
              <Button
                variant="secondary"
                onClick={() => setConfirmClose(false)}
              >
                {t("accountKeepEditing")}
              </Button>
              <Button variant="danger" onClick={onClose}>
                {t("accountDiscardAction")}
              </Button>
            </div>
          </Alert>
        )}
        <div className="flex justify-end border-t border-border pt-field">
          <Button type="submit" loading={busy} disabled={!dirty}>
            {t("accountSave")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
