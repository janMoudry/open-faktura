import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Alert, Button, Checkbox, Dialog, Icon } from "@open-faktura/ui";
import {
  inboxNotificationFeed,
  inboxNotificationConfigure,
  inboxNotificationRead,
  SalesFailure,
  type NotificationFeed,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
/** Remount keyed by company so open state and private feed cannot cross agendas. */
export function InboxNotifications(props: {
  companyId: string;
  onExpired: () => void;
}) {
  return <NotificationPanel key={props.companyId} {...props} />;
}
function NotificationPanel({
  companyId,
  onExpired,
}: {
  companyId: string;
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const [feed, setFeed] = useState<NotificationFeed | null>(null),
    [open, setOpen] = useState(false),
    [busy, setBusy] = useState(false),
    [failed, setFailed] = useState(false),
    [documents, setDocuments] = useState(false),
    [errors, setErrors] = useState(false);
  const apply = (f: NotificationFeed) => {
    setFeed(f);
    setDocuments(f.settings.new_documents);
    setErrors(f.settings.errors);
  };
  useEffect(() => {
    let active = true;
    let pending = false;
    const refresh = async () => {
      if (pending) return;
      pending = true;
      try {
        const f = await inboxNotificationFeed(apiUrl, companyId);
        if (active) {
          apply(f);
          setFailed(false);
        }
      } catch (e) {
        if (active) {
          setFailed(true);
          if (e instanceof SalesFailure && [401, 403, 404].includes(e.status))
            setFeed(null);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      } finally {
        pending = false;
      }
    };
    void refresh();
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible" && !open) void refresh();
    }, 60000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [companyId, onExpired, open]);
  async function action(task: () => Promise<NotificationFeed>) {
    setBusy(true);
    setFailed(false);
    try {
      apply(await task());
    } catch (e) {
      setFailed(true);
      if (e instanceof SalesFailure && [401, 403, 404].includes(e.status))
        setFeed(null);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        aria-label={t("inboxNotificationsCount", { count: feed?.unread ?? 0 })}
        onClick={() => setOpen(true)}
      >
        <Icon name="bell" />
        {!!feed?.unread && <span>{feed.unread}</span>}
      </Button>
      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        title={t("inboxNotifications")}
        description={t("inboxNotificationsDescription")}
      >
        <div className="space-y-field">
          {failed && (
            <Alert role="alert" tone="danger">
              {t("inboxNotificationsFailed")}
            </Alert>
          )}
          <fieldset disabled={busy || !feed} className="space-y-control">
            <legend>{t("inboxNotificationsCategories")}</legend>
            <Checkbox
              label={t("inboxNotificationsDocuments")}
              checked={documents}
              onChange={(e) => setDocuments(e.target.checked)}
            />
            <Checkbox
              label={t("inboxNotificationsErrors")}
              checked={errors}
              onChange={(e) => setErrors(e.target.checked)}
            />
            <Button
              disabled={
                !feed ||
                busy ||
                (documents === feed.settings.new_documents &&
                  errors === feed.settings.errors)
              }
              onClick={() =>
                feed &&
                void action(() =>
                  inboxNotificationConfigure(apiUrl, companyId, {
                    revision: feed.settings.revision,
                    new_documents: documents,
                    errors,
                  }),
                )
              }
            >
              {t("inboxNotificationsSave")}
            </Button>
          </fieldset>
          <div className="flex flex-wrap gap-control">
            <Button
              disabled={busy}
              onClick={() =>
                void action(() => inboxNotificationFeed(apiUrl, companyId))
              }
            >
              {t("inboxNotificationsRefresh")}
            </Button>
            <Button
              disabled={busy || !feed || feed.unread === 0}
              onClick={() =>
                feed &&
                void action(() =>
                  inboxNotificationRead(apiUrl, companyId, feed.through),
                )
              }
            >
              {t("inboxNotificationsRead")}
            </Button>
          </div>
          {feed && feed.events.length === 0 && (
            <p>{t("inboxNotificationsEmpty")}</p>
          )}
          <ul className="space-y-control">
            {feed?.events.map((e) => (
              <li
                key={e.id}
                className="rounded-control border border-border p-control"
              >
                {e.unread && (
                  <span className="font-semibold">
                    {t("inboxNotificationsUnread")}{" "}
                  </span>
                )}
                <Link
                  to={`/companies/${encodeURIComponent(companyId)}/inbox`}
                  onClick={() => setOpen(false)}
                >
                  {e.category === "document"
                    ? t("inboxNotificationsDocument", {
                        filename: e.filename ?? "",
                      })
                    : t(
                        e.code === "message_rejected"
                          ? "inboxNotificationsRejected"
                          : e.code === "gateway_failed"
                            ? "inboxNotificationsGateway"
                            : "inboxNotificationsMailbox",
                      )}
                </Link>
                <time
                  className="block text-sm text-muted"
                  dateTime={e.created_at}
                >
                  {new Intl.DateTimeFormat(language, {
                    dateStyle: "short",
                    timeStyle: "short",
                  }).format(new Date(e.created_at))}
                </time>
              </li>
            ))}
          </ul>
        </div>
      </Dialog>
    </>
  );
}
