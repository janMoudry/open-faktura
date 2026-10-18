import { useEffect, useState } from "react";
import { Alert, Button, Heading, Select } from "@open-faktura/ui";
import {
  cashSettings,
  SalesFailure,
  type CashSettings,
  type CashAccount,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useCashCommand } from "../../hooks/useCashCommand";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
export function CashAutomation({
  companyId,
  accounts,
  canWrite,
  version,
}: {
  companyId: string;
  accounts: CashAccount[];
  canWrite: boolean;
  version: number;
}) {
  const { t } = useAppTranslation(),
    { onExpired } = useWorkspace(),
    cmd = useCashCommand(companyId);
  const [value, setValue] = useState<CashSettings | null>(null),
    [choice, setChoice] = useState(""),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0),
    [saved, setSaved] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    setError(false);
    cashSettings(apiUrl, companyId, controller.signal)
      .then((v) => {
        if (!controller.signal.aborted) {
          setValue(v);
          setChoice(v.account_id ?? "");
        }
      })
      .catch((e) => {
        if (!controller.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => controller.abort();
  }, [companyId, version, attempt]);
  return (
    <section
      className="workspace-panel p-field space-y-field"
      aria-label={t("cashAutomation")}
    >
      <Heading level={2} size="section">
        {t("cashAutomation")}
      </Heading>
      <p className="text-sm text-muted">{t("cashAutomationHelp")}</p>
      {error ? (
        <Alert tone="danger">
          {t("cashFailed")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((x) => x + 1)}>
            {t("cashRetry")}
          </Button>
        </Alert>
      ) : !value ? (
        <p role="status">{t("cashLoading")}</p>
      ) : (
        <form
          className="flex flex-wrap items-end gap-field"
          onSubmit={async (e) => {
            e.preventDefault();
            if (cmd.busy) return;
            setSaved(false);
            const r = await cmd.execute({
              request_id: "",
              action: "settings",
              account_id: choice || null,
              revision: value.revision,
            });
            if (r?.kind === "settings") {
              setValue(r.value);
              setSaved(true);
            }
          }}
        >
          <Select
            label={t("cashDefault")}
            value={choice}
            disabled={!canWrite || cmd.busy}
            onChange={(v) => {
              setChoice(v);
              setSaved(false);
            }}
            options={[
              { value: "", label: t("cashAutomationOff") },
              ...accounts
                .filter((a) => !a.archived)
                .map((a) => ({ value: a.id, label: a.data.name })),
            ]}
          />
          {canWrite && (
            <Button
              variant="secondary"
              type="submit"
              disabled={cmd.busy || choice === (value.account_id ?? "")}
            >
              {t(cmd.busy ? "cashSaving" : "cashSave")}
            </Button>
          )}
          {cmd.error && (
            <Alert tone="danger">
              {t(cmd.error)}{" "}
              <Button
                type="button"
                variant="secondary"
                onClick={() => {
                  cmd.setError("");
                  setAttempt((x) => x + 1);
                }}
              >
                {t("cashRetry")}
              </Button>
            </Alert>
          )}
          {saved && <Alert tone="success">{t("cashSaved")}</Alert>}
        </form>
      )}
    </section>
  );
}
