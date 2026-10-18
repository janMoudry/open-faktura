import { useRef, useState } from "react";
import { Alert, Button, Dialog, Field } from "@open-faktura/ui";
import {
  anonymizeContact,
  ContactFailure,
  type Contact,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
export function ContactAnonymization({
  company,
  contact,
  disabled,
  onChanged,
  onExpired,
}: {
  company: string;
  contact: Contact;
  disabled: boolean;
  onChanged: (contact: Contact) => void;
  onExpired: () => void;
}) {
  const { t } = useAppTranslation();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const request = useRef(crypto.randomUUID());
  async function execute() {
    if (busy || name !== contact.data.name) return;
    setBusy(true);
    setError(false);
    try {
      const value = await anonymizeContact(apiUrl, company, contact.id, {
        request_id: request.current,
        revision: contact.revision,
        confirm: true,
      });
      onChanged(value);
      setOpen(false);
      setName("");
    } catch (e) {
      if (e instanceof ContactFailure && e.status === 401) onExpired();
      else setError(true);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        disabled={disabled}
        onClick={() => {
          request.current = crypto.randomUUID();
          setName("");
          setError(false);
          setOpen(true);
        }}
      >
        {t("contactAnonymize")}
      </Button>
      <Dialog
        open={open}
        title={t("contactAnonymizeTitle")}
        description={t("contactAnonymizeIrreversible")}
        onClose={() => {
          if (!busy) setOpen(false);
        }}
      >
        <div className="space-y-field">
          <Alert tone="warning">{t("contactAnonymizeScope")}</Alert>
          <Field
            label={t("contactAnonymizeName")}
            hint={contact.data.name}
            value={name}
            disabled={busy}
            autoComplete="off"
            onChange={(e) => setName(e.target.value)}
          />
          {error && (
            <Alert tone="danger" role="alert">
              {t("contactAnonymizeError")}
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
            <Button
              disabled={name !== contact.data.name}
              loading={busy}
              onClick={() => void execute()}
            >
              {t("contactAnonymizeConfirm")}
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
