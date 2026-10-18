import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Checkbox,
  Dialog,
  Field,
  TextareaField,
} from "@open-faktura/ui";
import {
  inboxOriginalHistory,
  inboxOriginalSend,
  SalesFailure,
  type InboxDocument,
  type InboxOriginalHistory,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
const status: Record<string, AppKey> = {
  queued: "quoteQueued",
  sending: "quoteSending",
  sent: "quoteSent",
  failed: "quoteFailed",
  uncertain: "quoteUncertain",
};
export function InboxOriginalDelivery({
  companyId,
  document: doc,
  write,
  onExpired,
}: {
  companyId: string;
  document: InboxDocument;
  write: boolean;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [open, setOpen] = useState(false),
    [history, setHistory] = useState<InboxOriginalHistory | null>(null),
    [recipient, setRecipient] = useState(""),
    [subject, setSubject] = useState(""),
    [body, setBody] = useState(""),
    [confirmed, setConfirmed] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(false),
    [queued, setQueued] = useState(false);
  const attempt = useRef({ signature: "", id: "" });
  function failure(e: unknown) {
    setError(true);
    if (e instanceof SalesFailure && [401, 403, 404].includes(e.status)) {
      setHistory(null);
      setConfirmed(false);
    }
    if (e instanceof SalesFailure && e.status === 401) onExpired();
  }
  useEffect(() => {
    if (!open) return;
    let active = true;
    setBusy(true);
    inboxOriginalHistory(apiUrl, companyId, doc.id)
      .then((h) => {
        if (active) setHistory(h);
      })
      .catch((e) => {
        if (active) failure(e);
      })
      .finally(() => {
        if (active) setBusy(false);
      });
    return () => {
      active = false;
    };
  }, [open, companyId, doc.id]);
  async function refresh() {
    setBusy(true);
    setError(false);
    try {
      setHistory(await inboxOriginalHistory(apiUrl, companyId, doc.id));
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  async function send() {
    const payload = {
      revision: doc.revision,
      recipient,
      subject,
      body,
      confirm_original: confirmed,
    };
    const signature = JSON.stringify(payload);
    if (attempt.current.signature !== signature)
      attempt.current = { signature, id: crypto.randomUUID() };
    setBusy(true);
    setError(false);
    try {
      await inboxOriginalSend(apiUrl, companyId, doc.id, {
        ...payload,
        request_id: attempt.current.id,
      });
      setQueued(true);
      setConfirmed(false);
      setHistory(await inboxOriginalHistory(apiUrl, companyId, doc.id));
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <Button
        variant="secondary"
        disabled={busy}
        onClick={() => {
          if (queued) attempt.current = { signature: "", id: "" };
          setHistory(null);
          setOpen(true);
          setError(false);
          setQueued(false);
          setConfirmed(false);
          if (!subject)
            setSubject(t("inboxOriginalSubject", { filename: doc.filename }));
          if (!body) setBody(t("inboxOriginalBody"));
        }}
      >
        {t(write ? "inboxOriginalEmail" : "inboxOriginalEmailHistory")}
      </Button>
      <Dialog
        open={open}
        onClose={() => {
          if (!busy) setOpen(false);
        }}
        title={t(write ? "inboxOriginalEmail" : "inboxOriginalEmailHistory")}
        description={t("inboxOriginalEmailDescription")}
      >
        <div className="space-y-field">
          {error && (
            <Alert role="alert" tone="danger">
              {t("inboxOriginalEmailFailed")}
            </Alert>
          )}
          {busy && <p role="status">{t("cashLoading")}</p>}
          {queued && <p role="status">{t("quoteQueuedNotice")}</p>}
          <p className="break-all">
            {doc.filename} · {doc.size} B<br />
            {t("inboxOriginalHash")}: {doc.sha256}
          </p>
          <p>
            {t("quoteFrom")}:{" "}
            {history
              ? (history.sender ?? t("inboxOriginalSmtpMissing"))
              : t("cashLoading")}
          </p>
          {write && (
            <>
              <Field
                label={t("quoteTo")}
                type="email"
                maxLength={254}
                required
                value={recipient}
                disabled={busy}
                onChange={(e) => {
                  setRecipient(e.target.value);
                  setConfirmed(false);
                }}
              />
              <Field
                label={t("quoteSubject")}
                maxLength={200}
                required
                value={subject}
                disabled={busy}
                onChange={(e) => {
                  setSubject(e.target.value);
                  setConfirmed(false);
                }}
              />
              <TextareaField
                label={t("quoteBody")}
                maxLength={20000}
                rows={5}
                required
                value={body}
                disabled={busy}
                onChange={(e) => {
                  setBody(e.target.value);
                  setConfirmed(false);
                }}
              />
              <Checkbox
                label={t("inboxOriginalConfirm", {
                  filename: doc.filename,
                  recipient,
                })}
                checked={confirmed}
                disabled={busy || !recipient}
                onChange={(e) => setConfirmed(e.target.checked)}
              />
              <Button
                disabled={
                  busy ||
                  !history?.configured ||
                  !confirmed ||
                  !recipient ||
                  !subject.trim() ||
                  !body.trim()
                }
                onClick={() => void send()}
              >
                {t("inboxOriginalQueue")}
              </Button>
            </>
          )}
          <p className="text-sm text-muted">{t("quoteSentHint")}</p>
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => void refresh()}
          >
            {t("inboxOriginalHistoryRefresh")}
          </Button>
          <ul className="space-y-control">
            {history?.items.map((d) => (
              <li
                key={d.id}
                className="border border-border rounded-control p-control"
              >
                <Badge>{t(status[d.status] ?? "quoteUncertain")}</Badge>
                <p className="break-all">
                  {d.recipient} · {d.filename}
                </p>
                <p>{new Date(d.created_at).toLocaleString(language)}</p>
                {d.error_code && (
                  <p>
                    {t(
                      d.error_code === "permission_changed"
                        ? "inboxOriginalPermissionChanged"
                        : "inboxOriginalDeliveryProblem",
                    )}
                  </p>
                )}
              </li>
            ))}
          </ul>
        </div>
      </Dialog>
    </>
  );
}
