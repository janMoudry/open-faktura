import { useEffect, useState } from "react";
import { Alert, Button, Dialog } from "@open-faktura/ui";
import {
  bankBatchPreview,
  SalesFailure,
  type BankBatchPreview,
  type BankAccount,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { useBankCommand } from "../../hooks/useBankCommand";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
const statuses: Record<BankBatchPreview["items"][number]["status"], AppKey> = {
  ready: "bankBatchReady",
  matched: "bankBatchMatched",
  archived: "bankBatchArchived",
  no_symbol: "bankBatchNoSymbol",
  no_match: "bankBatchNoMatch",
  ambiguous: "bankBatchAmbiguous",
  partial: "bankBatchPartial",
  competing_movement: "bankBatchCompeting",
  shared_target: "bankBatchShared",
  too_many: "bankBatchTooMany",
};
export function BankBatchDialog({
  companyId,
  movementIds,
  accounts,
  onClose,
  onSaved,
}: {
  companyId: string;
  movementIds: string[];
  accounts: BankAccount[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t, language } = useAppTranslation();
  const { onExpired } = useWorkspace();
  const command = useBankCommand(companyId);
  const [preview, setPreview] = useState<BankBatchPreview | null>(null);
  const [failed, setFailed] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    setPreview(null);
    setFailed(false);
    command.setError("");
    bankBatchPreview(apiUrl, companyId, movementIds, controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) setPreview(value);
      })
      .catch((e) => {
        if (!controller.signal.aborted) {
          setFailed(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => controller.abort();
  }, [companyId, movementIds, attempt]);
  const stale = command.error === "bankConflict";
  return (
    <Dialog
      open
      title={t("bankBatchTitle")}
      description={t("bankBatchHelp")}
      onClose={() => {
        if (!command.busy) onClose();
      }}
    >
      <div className="space-y-field">
        {command.error && (
          <Alert tone="danger">
            {t(stale ? "bankBatchStale" : command.error)}
          </Alert>
        )}
        {failed ? (
          <Alert tone="danger">{t("bankFailed")}</Alert>
        ) : !preview ? (
          <p role="status">{t("bankLoading")}</p>
        ) : (
          <>
            <p role="status">
              {t("bankBatchCount", {
                count: preview.ready_count,
                skipped: preview.items.length - preview.ready_count,
              })}
            </p>
            <div
              className="max-h-80 overflow-y-auto rounded-control border border-border"
              tabIndex={0}
              role="region"
              aria-label={t("bankBatchResult")}
            >
              <ul className="divide-y divide-border">
                {[...preview.items]
                  .sort(
                    (a, b) =>
                      Number(b.status === "ready") -
                      Number(a.status === "ready"),
                  )
                  .map((item) => (
                    <li
                      key={item.movement.id}
                      className="p-control space-y-inline"
                    >
                      <div className="flex flex-wrap items-center justify-between gap-control">
                        <span className="font-medium">
                          {item.movement.number}
                        </span>
                        <span className="font-medium whitespace-nowrap">
                          {moneyText(
                            item.movement.data.amount,
                            language,
                            item.movement.data.currency,
                          )}
                        </span>
                      </div>
                      <p className="text-sm text-muted break-words">
                        {
                          accounts.find(
                            (a) => a.id === item.movement.account_id,
                          )?.data.name
                        }{" "}
                        · {t("bankVs")}:{" "}
                        {item.movement.data.variable_symbol || "—"}
                      </p>
                      {item.candidate && (
                        <p className="text-sm break-words">
                          <span className="font-medium">
                            {item.candidate.number}
                          </span>{" "}
                          · {item.candidate.partner}
                        </p>
                      )}
                      <p className="text-sm">{t(statuses[item.status])}</p>
                      {item.candidate && (
                        <p className="text-sm text-muted">
                          {t(
                            item.candidate.payment_id
                              ? "bankSuggestExisting"
                              : "bankCreatePayment",
                          )}
                        </p>
                      )}
                    </li>
                  ))}
              </ul>
            </div>
            {preview.ready_count === 0 && <Alert>{t("bankBatchEmpty")}</Alert>}
          </>
        )}
        <div className="flex flex-wrap justify-end gap-control">
          <Button variant="secondary" disabled={command.busy} onClick={onClose}>
            {t("bankClose")}
          </Button>
          <Button
            variant="secondary"
            disabled={command.busy || (!preview && !failed)}
            onClick={() => setAttempt((v) => v + 1)}
          >
            {t("bankBatchRefresh")}
          </Button>
          <Button
            disabled={command.busy || !preview?.ready_count || stale || failed}
            onClick={async () => {
              if (!preview) return;
              const result = await command.execute({
                request_id: "",
                action: "batch_match",
                movement_ids: movementIds,
                fingerprint: preview.fingerprint,
              });
              if (result?.kind === "batch") onSaved();
            }}
          >
            {t(command.busy ? "bankSaving" : "bankBatchConfirm", {
              count: preview?.ready_count ?? 0,
            })}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
