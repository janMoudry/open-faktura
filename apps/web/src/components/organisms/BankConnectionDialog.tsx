import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Dialog,
  Field,
  Select,
  Switch,
  Heading,
} from "@open-faktura/ui";
import {
  bankConnectionStatus,
  bankConnectionExecute,
  SalesFailure,
  type BankAccount,
  type BankConnectionStatus,
  type BankConnectionCommand,
} from "@open-faktura/api-client";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
export function BankConnectionDialog({
  companyId,
  accounts,
  onClose,
  onSynced,
}: {
  companyId: string;
  accounts: BankAccount[];
  onClose: () => void;
  onSynced: () => void;
}) {
  const { t } = useAppTranslation();
  const [selected, setSelected] = useState(accounts[0]?.id ?? "");
  const [busy, setBusy] = useState(false);
  return (
    <Dialog
      open
      description={t("connectionHelp")}
      title={t("connectionTitle")}
      onClose={busy ? () => {} : onClose}
    >
      <div className="space-y-field">
        <Select
          disabled={busy}
          label={t("bankAccount")}
          value={selected}
          onChange={setSelected}
          options={accounts.map((a) => ({ value: a.id, label: a.data.name }))}
        />
        {accounts.find((a) => a.id === selected) && (
          <Connection
            key={selected}
            account={accounts.find((a) => a.id === selected)!}
            companyId={companyId}
            onBusy={setBusy}
            onSynced={onSynced}
          />
        )}
        <Button variant="secondary" disabled={busy} onClick={onClose}>
          {t("bankClose")}
        </Button>
      </div>
    </Dialog>
  );
}
function Connection({
  companyId,
  account,
  onBusy,
  onSynced,
}: {
  companyId: string;
  account: BankAccount;
  onBusy: (v: boolean) => void;
  onSynced: () => void;
}) {
  const { t, language } = useAppTranslation(),
    { company, onExpired, setPageDirty } = useWorkspace();
  const [status, setStatus] = useState<BankConnectionStatus | null>(null),
    [provider, setProvider] = useState("fio"),
    [secret, setSecret] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<AppKey | "">(""),
    [attempt, setAttempt] = useState(0),
    [confirmDisconnect, setConfirmDisconnect] = useState(false);
  const editable = company?.id === companyId && company.role !== "viewer";
  useEffect(() => {
    setPageDirty(secret.length > 0);
    return () => setPageDirty(false);
  }, [secret, setPageDirty]);
  useEffect(() => {
    const c = new AbortController();
    setError("");
    setStatus(null);
    bankConnectionStatus(apiUrl, companyId, account.id, c.signal)
      .then((s) => {
        if (!c.signal.aborted) {
          setStatus(s);
          setProvider(s.provider ?? s.providers[0]?.id ?? "");
        }
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError("connectionLoadError");
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, account.id, attempt]);
  const [matchingEnabled, setMatchingEnabled] = useState(false);
  const [matchingTreasury, setMatchingTreasury] = useState(false);
  useEffect(() => {
    if (status) {
      setMatchingEnabled(status.matching_enabled);
      setMatchingTreasury(status.matching_treasury);
    }
  }, [status]);
  const [interval, setInterval] = useState("0");
  useEffect(() => {
    if (status)
      setInterval(
        status.schedule_enabled ? String(status.interval_minutes) : "0",
      );
  }, [status]);
  const run = async (action: BankConnectionCommand["action"]) => {
    if (!status || busy) return;
    setBusy(true);
    onBusy(true);
    setError("");
    try {
      const base = {
        account_id: account.id,
        revision: status.revision,
        request_id: crypto.randomUUID(),
      };
      const result = await bankConnectionExecute(
        apiUrl,
        companyId,
        action === "save"
          ? { ...base, action, provider, secret: secret || null }
          : action === "schedule"
            ? {
                ...base,
                action,
                enabled: interval !== "0",
                interval_minutes:
                  Number(interval || "60") === 0
                    ? 60
                    : (Number(interval) as 15 | 60 | 360 | 1440),
              }
            : action === "matching"
              ? {
                  ...base,
                  action,
                  enabled: matchingEnabled,
                  include_treasury: matchingTreasury,
                }
              : { ...base, action },
      );
      setStatus(result);
      setSecret("");
      setConfirmDisconnect(false);
      if (action === "sync" && result.last_run?.status === "success")
        onSynced();
    } catch (e) {
      if (e instanceof SalesFailure && e.status === 401) onExpired();
      setError(
        e instanceof SalesFailure && e.status === 409
          ? "connectionConflict"
          : "connectionActionError",
      );
    } finally {
      setBusy(false);
      onBusy(false);
    }
  };
  const date = (v: string | null) =>
    v
      ? new Intl.DateTimeFormat(language, {
          dateStyle: "medium",
          timeStyle: "short",
        }).format(new Date(v))
      : t("connectionNever");
  const errorLabel: Record<string, AppKey> = {
    connection_network: "connectionNetwork",
    connection_auth: "connectionAuth",
    connection_rate: "connectionRate",
    connection_size: "connectionFormat",
    connection_format: "connectionFormat",
    connection_account: "connectionAccount",
    connection_conflict: "connectionImportConflict",
    connection_interrupted: "connectionInterrupted",
  };
  return (
    <div className="space-y-field">
      {error && <Alert tone="danger">{t(error)}</Alert>}
      {!status ? (
        <>
          <p role="status">{t("bankLoading")}</p>
          <Button variant="secondary" onClick={() => setAttempt((x) => x + 1)}>
            {t("bankRetry")}
          </Button>
        </>
      ) : (
        <>
          <dl className="grid gap-field sm:grid-cols-2">
            <div>
              <dt className="text-sm text-muted">{t("connectionState")}</dt>
              <dd>
                {t(
                  status.configured
                    ? "connectionConfigured"
                    : "connectionDisconnected",
                )}
              </dd>
            </div>
            <div>
              <dt className="text-sm text-muted">{t("connectionLastSync")}</dt>
              <dd>{date(status.last_sync_at)}</dd>
            </div>
          </dl>
          <div className="space-y-control">
            <p className="text-sm">
              {t(
                status.schedule_enabled
                  ? "connectionAutoOn"
                  : "connectionAutoOff",
              )}
            </p>
            {status.next_run_at && (
              <p className="text-sm text-muted">
                {t("connectionNextRun")}: {date(status.next_run_at)}
              </p>
            )}
            {status.schedule_error && (
              <Alert tone="danger">
                {t(
                  status.schedule_error === "connection_permission"
                    ? "connectionAutoPermission"
                    : status.schedule_error === "connection_busy"
                      ? "connectionAutoBusy"
                      : "connectionAutoSetup",
                )}
              </Alert>
            )}
          </div>
          {status.last_run && (
            <Alert
              tone={
                status.last_run.status === "failed"
                  ? "danger"
                  : status.last_run.status === "success"
                    ? "success"
                    : "neutral"
              }
            >
              <p>
                {t(
                  (
                    {
                      save: "connectionSaved",
                      schedule: "connectionScheduleSaved",
                      matching: "matchingSaved",
                      disconnect: "connectionRemoved",
                      test: "connectionTestResult",
                      sync: "connectionSyncResult",
                    } as const
                  )[status.last_run.kind],
                )}
                :{" "}
                {t(
                  (
                    {
                      success: "connectionSuccess",
                      failed: "connectionFailed",
                      running: "connectionRunning",
                    } as const
                  )[status.last_run.status],
                )}
              </p>
              <p className="text-sm">
                {date(
                  status.last_run.finished_at ?? status.last_run.started_at,
                )}
              </p>
              {status.last_run.error_code && (
                <p>
                  {t(
                    errorLabel[status.last_run.error_code] ??
                      "connectionActionError",
                  )}
                </p>
              )}
              {status.last_run.kind === "sync" &&
                status.last_run.status === "success" && (
                  <p>
                    {t("connectionCounts", {
                      count: status.last_run.imported,
                      duplicates: status.last_run.duplicates,
                    })}
                  </p>
                )}
            </Alert>
          )}
          <section
            className="space-y-field border-t border-border pt-field"
            aria-label={t("matchingTitle")}
          >
            <Heading level={3} size="card">
              {t("matchingTitle")}
            </Heading>
            <p className="text-sm text-muted">{t("matchingHelp")}</p>
            <p className="text-sm">
              {t(status.matching_enabled ? "matchingOn" : "matchingOff")}
            </p>
            {status.matching_next_at && (
              <p className="text-sm text-muted">
                {t("connectionNextRun")}: {date(status.matching_next_at)}
              </p>
            )}
            {status.matching_last && (
              <p className="text-sm">
                {t("matchingCounts", {
                  checked: status.matching_last.checked,
                  matched: status.matching_last.matched,
                })}{" "}
                · {date(status.matching_last.at)}
              </p>
            )}
            {status.matching_error && (
              <Alert tone="danger">
                {t(
                  status.matching_error === "matching_permission"
                    ? "matchingPermission"
                    : "matchingFailed",
                )}
              </Alert>
            )}
            {editable && (
              <>
                <Switch
                  label={t("matchingEnable")}
                  checked={matchingEnabled}
                  onChange={(e) => setMatchingEnabled(e.target.checked)}
                  disabled={
                    busy ||
                    status.busy ||
                    (account.archived && !status.matching_enabled)
                  }
                />
                <Switch
                  label={t("matchingTreasury")}
                  checked={matchingTreasury}
                  onChange={(e) => setMatchingTreasury(e.target.checked)}
                  disabled={busy || status.busy || account.archived}
                />
                <p className="text-sm text-muted">
                  {t("matchingTreasuryHelp")}
                </p>
                <Button
                  variant="secondary"
                  disabled={
                    busy ||
                    status.busy ||
                    !!secret ||
                    (matchingEnabled === status.matching_enabled &&
                      matchingTreasury === status.matching_treasury) ||
                    (account.archived && matchingEnabled)
                  }
                  onClick={() => void run("matching")}
                >
                  {t("matchingSave")}
                </Button>
              </>
            )}
          </section>
          {editable && (
            <>
              {!account.archived && status.configured && (
                <div className="space-y-field">
                  <Select
                    label={t("connectionAutoInterval")}
                    value={interval}
                    onChange={setInterval}
                    disabled={busy || status.busy}
                    options={[
                      { value: "0", label: t("connectionAutoOff") },
                      { value: "15", label: t("connectionEvery15") },
                      { value: "60", label: t("connectionEvery60") },
                      { value: "360", label: t("connectionEvery360") },
                      { value: "1440", label: t("connectionEvery1440") },
                    ]}
                  />
                  <p className="text-sm text-muted">
                    {t("connectionAutoHelp")}
                  </p>
                  <Button
                    variant="secondary"
                    disabled={
                      busy ||
                      status.busy ||
                      !!secret ||
                      provider !== status.provider
                    }
                    onClick={() => void run("schedule")}
                  >
                    {t("connectionAutoSave")}
                  </Button>
                </div>
              )}
              {!account.archived && (
                <form
                  className="space-y-field"
                  onSubmit={(e) => {
                    e.preventDefault();
                    void run("save");
                  }}
                >
                  <Select
                    label={t("connectionProvider")}
                    value={provider}
                    onChange={(v) => {
                      setProvider(v);
                      setSecret("");
                    }}
                    disabled={busy || status.busy}
                    options={status.providers.map((p) => ({
                      value: p.id,
                      label: p.label,
                    }))}
                  />
                  <p className="text-sm text-muted">
                    {t(
                      provider === "fio"
                        ? "connectionFioHelp"
                        : "connectionGatewayHelp",
                    )}
                  </p>
                  <Field
                    label={t("connectionToken")}
                    type="password"
                    autoComplete="new-password"
                    required={
                      !status.configured || provider !== status.provider
                    }
                    value={secret}
                    disabled={busy || status.busy}
                    onChange={(e) => setSecret(e.target.value)}
                  />
                  <p className="text-sm text-muted">
                    {t("connectionSecretHelp")}
                  </p>
                  <Button
                    type="submit"
                    disabled={busy || status.busy || !provider}
                  >
                    {t(busy ? "bankSaving" : "bankSave")}
                  </Button>
                </form>
              )}
              <div className="flex flex-wrap gap-control">
                <Button
                  variant="secondary"
                  disabled={
                    busy ||
                    status.busy ||
                    !status.configured ||
                    account.archived ||
                    !!secret ||
                    provider !== status.provider
                  }
                  onClick={() => void run("test")}
                >
                  {t("connectionTest")}
                </Button>
                <Button
                  disabled={
                    busy ||
                    status.busy ||
                    !status.configured ||
                    account.archived ||
                    !!secret ||
                    provider !== status.provider
                  }
                  onClick={() => void run("sync")}
                >
                  {t("connectionSync")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={busy || status.busy || !status.configured}
                  onClick={() => setConfirmDisconnect(true)}
                >
                  {t("connectionDisconnect")}
                </Button>
              </div>
              {confirmDisconnect && (
                <Alert>
                  <p>{t("connectionDisconnectHelp")}</p>
                  <div className="flex gap-control">
                    <Button
                      disabled={busy}
                      onClick={() => void run("disconnect")}
                    >
                      {t("connectionDisconnectConfirm")}
                    </Button>
                    <Button
                      variant="secondary"
                      disabled={busy}
                      onClick={() => setConfirmDisconnect(false)}
                    >
                      {t("bankClose")}
                    </Button>
                  </div>
                </Alert>
              )}
            </>
          )}
          <Button
            variant="secondary"
            disabled={busy || !!secret}
            onClick={() => setAttempt((x) => x + 1)}
          >
            {t("connectionRefresh")}
          </Button>
        </>
      )}
    </div>
  );
}
