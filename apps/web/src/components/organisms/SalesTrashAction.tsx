import { useRef, useState } from "react";
import { Alert, Button, Dialog } from "@open-faktura/ui";
import {
  changeSalesTrash,
  SalesFailure,
  type SalesDocument,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
export function SalesTrashAction({
  company,
  record,
  disabled,
  onDone,
  onExpired,
}: {
  company: string;
  record: SalesDocument;
  disabled: boolean;
  onDone: () => void;
  onExpired: () => void;
}) {
  const { t } = useAppTranslation();
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const request = useRef(crypto.randomUUID());
  async function execute() {
    if (busy) return;
    setBusy(true);
    setError(false);
    try {
      await changeSalesTrash(apiUrl, company, {
        request_id: request.current,
        id: record.id,
        revision: record.revision,
        action: "trash",
      });
      setOpen(false);
      onDone();
    } catch (e) {
      if (e instanceof SalesFailure && e.status === 401) onExpired();
      else setError(true);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <Button
        variant="ghost"
        disabled={disabled}
        onClick={() => {
          request.current = crypto.randomUUID();
          setError(false);
          setOpen(true);
        }}
      >
        {t("salesTrashMove")}
      </Button>
      <Dialog
        open={open}
        title={t("salesTrashMove")}
        description={t("salesTrashConfirm")}
        onClose={() => {
          if (!busy) setOpen(false);
        }}
      >
        <div className="space-y-field">
          <p>{record.data.name}</p>
          <p className="text-sm text-muted">{t("salesTrashHelp")}</p>
          {error && (
            <Alert tone="danger" role="alert">
              {t("salesTrashError")}
            </Alert>
          )}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setOpen(false)}
            >
              {t("cancelAction")}
            </Button>
            <Button loading={busy} onClick={() => void execute()}>
              {t("salesTrashMove")}
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
