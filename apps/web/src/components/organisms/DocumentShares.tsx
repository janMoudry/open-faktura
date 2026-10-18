import { useEffect, useRef, useState } from "react";
import { Alert, Badge, Button, Dialog, Field, Heading } from "@open-faktura/ui";
import {
  createInvoiceShare,
  listInvoiceShares,
  revokeInvoiceShare,
  invoiceShareViews,
  type InvoiceDraft,
  createQuoteShare,
  listQuoteShares,
  revokeQuoteShare,
  quoteShareViews,
  SalesFailure,
  type SalesDocument,
  type ShareLink,
  type ShareLinks,
  type ShareViews,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
export function DocumentShares({
  document: doc,
  kind = "quote",
  canWrite,
  onSend,
}: {
  document: SalesDocument | InvoiceDraft;
  kind?: "quote" | "invoice";
  canWrite: boolean;
  onSend?: (link: ShareLink) => void;
}) {
  const createShare =
    kind === "invoice" ? createInvoiceShare : createQuoteShare;
  const listShares = kind === "invoice" ? listInvoiceShares : listQuoteShares;
  const revokeShare =
    kind === "invoice" ? revokeInvoiceShare : revokeQuoteShare;
  const shareViews = kind === "invoice" ? invoiceShareViews : quoteShareViews;
  const { t, language } = useAppTranslation(),
    { onExpired } = useWorkspace();
  const [links, setLinks] = useState<ShareLinks | null>(null),
    [page, setPage] = useState(1),
    [refresh, setRefresh] = useState(0),
    [error, setError] = useState<AppKey | "">(""),
    [notice, setNotice] = useState<AppKey | "">(""),
    [busy, setBusy] = useState(false),
    [creating, setCreating] = useState(false),
    [days, setDays] = useState("30"),
    [revoking, setRevoking] = useState<ShareLink | null>(null),
    [selected, setSelected] = useState<ShareLink | null>(null),
    [views, setViews] = useState<ShareViews | null>(null),
    [viewPage, setViewPage] = useState(1),
    [viewError, setViewError] = useState(false);
  const attempt = useRef<{
    request_id: string;
    revision: number;
    expires_days: number;
  } | null>(null);
  const fail = (e: unknown) => {
    if (e instanceof SalesFailure && e.status === 401) onExpired();
    else setError("shareError");
  };
  useEffect(() => {
    const abort = new AbortController();
    setError("");
    listShares(apiUrl, doc.company_id, doc.id, page, abort.signal)
      .then(setLinks)
      .catch((e) => {
        if (!abort.signal.aborted) fail(e);
      });
    return () => abort.abort();
  }, [doc.company_id, doc.id, page, refresh, kind]);
  useEffect(() => {
    if (!selected) return;
    const abort = new AbortController();
    setViews(null);
    setViewError(false);
    shareViews(
      apiUrl,
      doc.company_id,
      doc.id,
      selected.id,
      viewPage,
      abort.signal,
    )
      .then(setViews)
      .catch((e) => {
        if (!abort.signal.aborted) {
          setViewError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => abort.abort();
  }, [selected, viewPage, refresh, doc.company_id, doc.id, kind]);
  const date = (v: string) =>
    new Intl.DateTimeFormat(language === "cs" ? "cs-CZ" : "en-GB", {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(v));
  async function create() {
    setBusy(true);
    setError("");
    const expires_days = Number(days);
    if (
      !attempt.current ||
      attempt.current.revision !== doc.revision ||
      attempt.current.expires_days !== expires_days
    )
      attempt.current = {
        request_id: crypto.randomUUID(),
        revision: doc.revision,
        expires_days,
      };
    try {
      await createShare(apiUrl, doc.company_id, doc.id, attempt.current);
      setCreating(false);
      setPage(1);
      setRefresh((v) => v + 1);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  }
  async function revoke() {
    if (!revoking) return;
    setBusy(true);
    setError("");
    try {
      await revokeShare(apiUrl, doc.company_id, doc.id, revoking.id);
      setRevoking(null);
      setRefresh((v) => v + 1);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  }
  async function copy(link: ShareLink) {
    try {
      await navigator.clipboard.writeText(link.url);
      setNotice("shareCopied");
    } catch {
      setNotice("shareCopyFailed");
    }
  }
  return (
    <section className="invoice-sheet">
      <div className="flex flex-wrap items-center justify-between gap-field border-b border-border p-field">
        <Heading level={2} size="section">
          {t("shareTitle")}
        </Heading>
        <div className="flex gap-control">
          <Button variant="ghost" onClick={() => setRefresh((v) => v + 1)}>
            {t("quoteRefresh")}
          </Button>
          {canWrite && (
            <Button
              variant="secondary"
              disabled={!links?.configured}
              onClick={() => {
                attempt.current = null;
                setError("");
                setCreating(true);
              }}
            >
              {t("shareCreate")}
            </Button>
          )}
        </div>
      </div>
      <div className="space-y-field p-field">
        <p className="text-sm text-muted">
          {t(kind === "invoice" ? "invoiceShareHint" : "shareHint")}
        </p>
        {links && !links.configured && canWrite && (
          <Alert>{t("shareUnavailableConfig")}</Alert>
        )}
        {error && !creating && !revoking && (
          <Alert tone="danger" role="alert">
            {t(error)}
          </Alert>
        )}
        {notice && (
          <p role="status" className="text-sm">
            {t(notice)}
          </p>
        )}
        {!links && !error && <p role="status">{t("salesLoading")}</p>}
        {links && !links.items.length && (
          <p className="text-sm text-muted">{t("shareEmpty")}</p>
        )}
        {links?.items.map((link) => {
          const active =
            !link.revoked_at &&
            new Date(link.expires_at).getTime() > Date.now();
          return (
            <div
              key={link.id}
              className="space-y-control rounded-control border border-border p-field"
            >
              <div className="flex flex-wrap justify-between gap-inline">
                <Badge>
                  {t(
                    link.revoked_at
                      ? "shareRevoked"
                      : active
                        ? "shareActive"
                        : "shareExpired",
                  )}
                </Badge>
                <span className="text-sm text-muted">
                  {t("shareExpires")}: {date(link.expires_at)}
                </span>
              </div>
              <Field
                label={t("shareUrl")}
                value={link.url}
                readOnly
                onFocus={(e) => e.target.select()}
              />
              <dl className="flex flex-wrap gap-field text-sm">
                <div>
                  <dt className="text-muted">{t("shareViews")}</dt>
                  <dd>{link.view_count}</dd>
                </div>
                <div>
                  <dt className="text-muted">{t("shareFirst")}</dt>
                  <dd>
                    {link.first_viewed_at
                      ? date(link.first_viewed_at)
                      : t("shareNever")}
                  </dd>
                </div>
                <div>
                  <dt className="text-muted">{t("shareLast")}</dt>
                  <dd>
                    {link.last_viewed_at ? date(link.last_viewed_at) : "—"}
                  </dd>
                </div>
              </dl>
              <div className="flex flex-wrap gap-control">
                <Button
                  variant="secondary"
                  disabled={!active}
                  onClick={() => void copy(link)}
                >
                  {t("shareCopy")}
                </Button>
                {canWrite && onSend && (
                  <Button
                    variant="secondary"
                    disabled={!active}
                    onClick={() => onSend(link)}
                  >
                    {t("shareSendLink")}
                  </Button>
                )}
                <Button
                  variant="ghost"
                  onClick={() => {
                    setSelected(link);
                    setViewPage(1);
                  }}
                >
                  {t("shareHistory")}
                </Button>
                {canWrite && active && (
                  <Button
                    variant="ghost"
                    onClick={() => {
                      setError("");
                      setRevoking(link);
                    }}
                  >
                    {t("shareRevoke")}
                  </Button>
                )}
              </div>
            </div>
          );
        })}
        {links && links.total > 25 && (
          <div className="flex justify-end gap-control">
            <Button disabled={page === 1} onClick={() => setPage((v) => v - 1)}>
              {t("quotePrevious")}
            </Button>
            <span>{t("quotePage", { page })}</span>
            <Button
              disabled={page * 25 >= links.total}
              onClick={() => setPage((v) => v + 1)}
            >
              {t("quoteNext")}
            </Button>
          </div>
        )}
        <p className="text-sm text-muted">{t("shareTrackingHint")}</p>
      </div>
      <Dialog
        open={creating}
        onClose={() => {
          if (!busy) setCreating(false);
        }}
        title={t("shareCreate")}
        description={t(kind === "invoice" ? "invoiceShareHint" : "shareHint")}
      >
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            void create();
          }}
        >
          <Field
            label={t("shareExpiry")}
            type="number"
            min={1}
            max={365}
            step={1}
            required
            value={days}
            disabled={busy}
            onChange={(e) => setDays(e.target.value)}
          />
          {error && (
            <Alert tone="danger" role="alert">
              {t(error)}
            </Alert>
          )}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setCreating(false)}
            >
              {t("quoteCancel")}
            </Button>
            <Button type="submit" disabled={busy}>
              {t("shareCreate")}
            </Button>
          </div>
        </form>
      </Dialog>
      <Dialog
        open={!!revoking}
        onClose={() => {
          if (!busy) setRevoking(null);
        }}
        title={t("shareRevoke")}
        description={t("shareRevokeHint")}
      >
        <div className="space-y-field">
          {error && (
            <Alert tone="danger" role="alert">
              {t(error)}
            </Alert>
          )}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setRevoking(null)}
            >
              {t("quoteCancel")}
            </Button>
            <Button disabled={busy} onClick={() => void revoke()}>
              {t("shareRevoke")}
            </Button>
          </div>
        </div>
      </Dialog>
      <Dialog
        open={!!selected}
        onClose={() => setSelected(null)}
        title={t("shareHistory")}
        description={t("shareTrackingHint")}
      >
        <div className="space-y-field">
          {viewError ? (
            <Alert tone="danger">{t("shareError")}</Alert>
          ) : !views ? (
            <p role="status">{t("salesLoading")}</p>
          ) : !views.items.length ? (
            <p>{t("shareNever")}</p>
          ) : (
            <ul className="divide-y divide-border">
              {views.items.map((v, i) => (
                <li
                  key={i}
                  className="flex flex-wrap justify-between gap-control py-control"
                >
                  <time>{date(v.viewed_at)}</time>
                  <span>
                    {t("shareViewedVersion")} {v.revision}
                  </span>
                </li>
              ))}
            </ul>
          )}
          <div className="flex flex-wrap justify-end gap-control">
            <Button variant="ghost" onClick={() => setRefresh((v) => v + 1)}>
              {t("quoteRefresh")}
            </Button>
            <Button
              disabled={viewPage === 1}
              onClick={() => setViewPage((v) => v - 1)}
            >
              {t("quotePrevious")}
            </Button>
            <Button
              disabled={!views || viewPage * 25 >= views.total}
              onClick={() => setViewPage((v) => v + 1)}
            >
              {t("quoteNext")}
            </Button>
          </div>
        </div>
      </Dialog>
    </section>
  );
}
