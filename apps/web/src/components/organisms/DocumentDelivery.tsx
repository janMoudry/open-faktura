import { DocumentAttachments, saveAttachmentFile } from "./DocumentAttachments";
import { moneyText } from "../../utils/invoice";
import { QuoteShares } from "./QuoteShares";
import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  Alert,
  Badge,
  Button,
  Dialog,
  Field,
  Heading,
  Icon,
  Select,
  TextareaField,
} from "@open-faktura/ui";
import {
  invoiceHistory,
  invoiceDeliveryPdf,
  sendInvoice,
  type InvoiceDraft,
  listQuoteShares,
  quoteHistory,
  quotePdf,
  sendQuote,
  SalesFailure,
  type SalesDocument,
  type DeliveryHistory,
  type QuoteDelivery as Delivery,
  type SendQuote,
  type ShareLink,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import cs from "../../locales/cs.json";
import en from "../../locales/en.json";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
const failureReasons: Record<string, AppKey> = {
  smtp_rejected: "mailFailureRejected",
  smtp_unknown: "mailFailureUnknown",
  interrupted: "mailFailureInterrupted",
  profile_unavailable: "mailFailureProfile",
  link_unavailable: "mailFailureLink",
  message_invalid: "mailFailureInvalid",
};
const statuses: Record<Delivery["status"], AppKey> = {
  queued: "quoteQueued",
  sending: "quoteSending",
  sent: "quoteSent",
  failed: "quoteFailed",
  uncertain: "quoteUncertain",
};
type CommonProps = {
  canSend: boolean;
  onChanged?: () => void;
  children: ReactNode;
};
type Props = CommonProps &
  (
    | { quote: SalesDocument; invoice?: never }
    | { invoice: InvoiceDraft; quote?: never }
  );
export function DocumentDelivery({
  quote,
  invoice,
  canSend,
  onChanged,
  children,
}: Props) {
  const doc = quote ?? invoice!;
  const data = quote ? quote.data.invoice : invoice!.data;
  const description = quote
    ? quote.data.name
    : data.note || data.calculation.lines[0]?.description || "";
  const historyRequest = quote ? quoteHistory : invoiceHistory;
  const sendRequest = quote ? sendQuote : sendInvoice;
  const pdfRequest = quote ? quotePdf : invoiceDeliveryPdf;
  const { t, language } = useAppTranslation();
  const { onExpired } = useWorkspace();
  const [history, setHistory] = useState<DeliveryHistory | null>(null),
    [page, setPage] = useState(1),
    [refresh, setRefresh] = useState(0),
    [historyError, setHistoryError] = useState(false);
  useEffect(() => {
    const changed = () => setRefresh((v) => v + 1);
    window.addEventListener("smtp-settings-changed", changed);
    return () => window.removeEventListener("smtp-settings-changed", changed);
  }, []);
  const [compose, setCompose] = useState(false),
    [detail, setDetail] = useState<Delivery | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<AppKey | "">(""),
    [notice, setNotice] = useState(false);
  const [attachmentIds, setAttachmentIds] = useState<string[]>([]);
  const [recipient, setRecipient] = useState(""),
    [subject, setSubject] = useState(""),
    [body, setBody] = useState("");
  const [preview, setPreview] = useState<string | null>(null);
  const url = useRef<string | null>(null),
    attempt = useRef<SendQuote | null>(null);
  const onChange = useRef(onChanged);
  onChange.current = onChanged;
  const seen = useRef(new Map<string, string>());
  useEffect(
    () => () => {
      if (url.current) URL.revokeObjectURL(url.current);
    },
    [],
  );
  useEffect(() => {
    const abort = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    async function load() {
      try {
        const h = await historyRequest(
          apiUrl,
          doc.company_id,
          doc.id,
          page,
          abort.signal,
        );
        if (abort.signal.aborted) return;
        setHistory(h);
        if (
          h.items.length &&
          !h.items.some((v) => v.status === "queued" || v.status === "sending")
        )
          setNotice(false);
        setHistoryError(false);
        let changed = false;
        for (const item of h.items) {
          if (
            seen.current.has(item.id) &&
            seen.current.get(item.id) !== item.status &&
            item.status === "sent"
          )
            changed = true;
          seen.current.set(item.id, item.status);
        }
        if (changed) onChange.current?.();
        if (
          h.items.some((v) => v.status === "queued" || v.status === "sending")
        )
          timer = setTimeout(() => void load(), 2000);
      } catch (e) {
        if (abort.signal.aborted) return;
        if (e instanceof SalesFailure && e.status === 401) onExpired();
        else setHistoryError(true);
      }
    }
    void load();
    return () => {
      abort.abort();
      clearTimeout(timer);
    };
  }, [doc.company_id, doc.id, page, refresh, onExpired]);
  const [share, setShare] = useState<ShareLink | null>(null);
  const [available, setAvailable] = useState<ShareLink[]>([]);
  const defaults = useRef({ attachment: "", link: "" });
  function method(value: string) {
    const next = value === "link" ? (available[0] ?? null) : null;
    setShare(next);
    setBody((current) =>
      current === defaults.current.attachment ||
      current === defaults.current.link
        ? next
          ? defaults.current.link
          : defaults.current.attachment
        : current,
    );
  }
  function open(link?: ShareLink) {
    setShare(link ?? null);
    setAvailable(link ? [link] : []);
    if (quote)
      void listQuoteShares(apiUrl, doc.company_id, doc.id)
        .then((result) => {
          const active = result.items.filter(
            (v) =>
              !v.revoked_at && new Date(v.expires_at).getTime() > Date.now(),
          );
          setAvailable(
            link ? [link, ...active.filter((v) => v.id !== link.id)] : active,
          );
        })
        .catch(() => {});
    const strings = data.language === "en" ? en : cs;
    const values: Record<string, string> = {
      number: doc.number ?? "",
      amount: moneyText(
        doc.totals?.payable ?? "0",
        data.language,
        data.calculation.currency,
      ),
      name: description,
      date: data.due_on
        ? new Intl.DateTimeFormat(
            data.language === "en" ? "en-GB" : "cs-CZ",
          ).format(new Date(data.due_on + "T12:00:00Z"))
        : "",
      supplier: data.supplier.name,
    };
    const fill = (s: string) =>
      s.replace(/\{\{(\w+)\}\}/g, (_, key: string) => values[key] ?? "");
    setRecipient(data.customer?.email ?? "");
    setSubject(
      fill(
        quote ? strings.quoteDefaultSubject : strings.invoiceMailDefaultSubject,
      ),
    );
    defaults.current = {
      attachment: fill(
        quote ? strings.quoteDefaultBody : strings.invoiceMailDefaultBody,
      ),
      link: fill(strings.shareDefaultBody),
    };
    setBody(link ? defaults.current.link : defaults.current.attachment);
    attempt.current = null;
    setError("");
    setAttachmentIds([]);
    setCompose(true);
  }
  async function send() {
    setBusy(true);
    setError("");
    const values = {
      revision: doc.revision,
      attachment_ids: attachmentIds,
      recipient,
      subject,
      body,
      ...(share ? { share_id: share.id } : {}),
    };
    if (
      !attempt.current ||
      JSON.stringify({ ...attempt.current, request_id: undefined }) !==
        JSON.stringify(values)
    )
      attempt.current = { ...values, request_id: crypto.randomUUID() };
    try {
      const queued = await sendRequest(
        apiUrl,
        doc.company_id,
        doc.id,
        attempt.current,
      );
      seen.current.set(queued.id, queued.status);
      if (queued.status === "sent") onChange.current?.();
      setCompose(false);
      setNotice(true);
      setPage(1);
      setRefresh((v) => v + 1);
    } catch (e) {
      if (e instanceof SalesFailure && e.status === 401) onExpired();
      else
        setError(
          e instanceof SalesFailure && e.status === 409
            ? "salesConflict"
            : quote
              ? "quoteMailError"
              : "invoiceMailError",
        );
    } finally {
      setBusy(false);
    }
  }
  async function pdf(print = false, delivery?: string) {
    setBusy(true);
    setError("");
    try {
      const blob = await pdfRequest(apiUrl, doc.company_id, doc.id, delivery);
      if (url.current) URL.revokeObjectURL(url.current);
      url.current = URL.createObjectURL(blob);
      if (print) setPreview(url.current);
      else {
        const a = window.document.createElement("a");
        a.href = url.current;
        a.download = `${doc.number ?? "quotation"}${delivery ? `-${delivery}` : ""}.pdf`;
        a.click();
      }
    } catch (e) {
      if (e instanceof SalesFailure && e.status === 401) onExpired();
      else setError("quotePdfError");
    } finally {
      setBusy(false);
    }
  }
  const date = (v: string) =>
    new Intl.DateTimeFormat(language === "cs" ? "cs-CZ" : "en-GB", {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(v));
  return (
    <section className="space-y-field">
      <div className="flex flex-wrap gap-inline">
        <Button variant="secondary" disabled={busy} onClick={() => void pdf()}>
          <Icon name="download" />
          {t(quote ? "quotePdf" : "invoiceDownload")}
        </Button>
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() => void pdf(true)}
        >
          <Icon name="print" />
          {t(quote ? "quotePrint" : "invoicePrint")}
        </Button>
        {canSend && (
          <Button
            disabled={busy || !history?.configured}
            onClick={() => open()}
          >
            <Icon name="mail" />
            {t("quoteSend")}
          </Button>
        )}
      </div>
      {history && !history.configured && canSend && (
        <Alert>{t("quoteSmtpMissing")}</Alert>
      )}
      {notice && (
        <Alert tone="success" role="status">
          {t("quoteQueuedNotice")}
        </Alert>
      )}
      {error && !compose && (
        <Alert tone="danger" role="alert">
          {t(error)}
        </Alert>
      )}
      {children}
      {quote && (
        <QuoteShares
          document={quote}
          canWrite={canSend}
          onSend={history?.configured ? (link) => open(link) : undefined}
        />
      )}
      <div className="invoice-sheet">
        <div className="flex flex-wrap items-center justify-between gap-field border-b border-border p-field">
          <Heading level={2} size="section">
            {t("quoteHistory")}
          </Heading>
          <Button variant="ghost" onClick={() => setRefresh((v) => v + 1)}>
            {t("quoteRefresh")}
          </Button>
        </div>
        {historyError ? (
          <Alert tone="danger">{t("quoteHistoryError")}</Alert>
        ) : !history ? (
          <p className="p-field" role="status">
            {t("salesLoading")}
          </p>
        ) : !history.items.length ? (
          <p className="p-field text-sm text-muted">
            {t(quote ? "quoteNoHistory" : "invoiceNoHistory")}
          </p>
        ) : (
          <>
            <ul className="divide-y divide-border md:hidden">
              {history.items.map((item) => (
                <li key={item.id} className="space-y-control p-field text-sm">
                  <div className="flex flex-wrap justify-between gap-inline">
                    <time className="text-muted">{date(item.created_at)}</time>
                    <Badge
                      tone={
                        item.status === "failed"
                          ? "danger"
                          : item.status === "uncertain"
                            ? "warning"
                            : "neutral"
                      }
                    >
                      {t(statuses[item.status])}
                    </Badge>
                  </div>
                  <p className="break-words font-medium">{item.subject}</p>
                  <p className="break-all text-muted">{item.recipient}</p>
                  <Button variant="secondary" onClick={() => setDetail(item)}>
                    {t("quoteMailDetail")}
                  </Button>
                </li>
              ))}
            </ul>
            <div className="hidden overflow-x-auto md:block">
              <table className="w-full min-w-invoice-register text-sm">
                <thead className="border-b border-border text-left text-muted">
                  <tr>
                    {[
                      "quoteMailDate",
                      "quoteTo",
                      "quoteSubject",
                      "quoteMailStatus",
                    ].map((k) => (
                      <th className="px-field py-control font-medium" key={k}>
                        {t(k as AppKey)}
                      </th>
                    ))}
                    <th className="px-field py-control">
                      <span className="sr-only">{t("quoteMailDetail")}</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {history.items.map((item) => (
                    <tr
                      key={item.id}
                      className="border-b border-border last:border-0"
                    >
                      <td className="whitespace-nowrap px-field py-field">
                        {date(item.created_at)}
                      </td>
                      <td className="px-field py-field">{item.recipient}</td>
                      <td className="px-field py-field">{item.subject}</td>
                      <td className="px-field py-field">
                        <Badge
                          tone={
                            item.status === "failed"
                              ? "danger"
                              : item.status === "uncertain"
                                ? "warning"
                                : "neutral"
                          }
                        >
                          {t(statuses[item.status])}
                        </Badge>
                      </td>
                      <td className="px-field py-field">
                        <Button
                          variant="ghost"
                          className="whitespace-nowrap"
                          onClick={() => setDetail(item)}
                        >
                          {t("quoteMailDetail")}
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        )}
        {history && history.total > 25 && (
          <div className="flex items-center justify-end gap-control border-t border-border p-field">
            <Button disabled={page === 1} onClick={() => setPage((v) => v - 1)}>
              {t("quotePrevious")}
            </Button>
            <span>{t("quotePage", { page })}</span>
            <Button
              disabled={page * 25 >= history.total}
              onClick={() => setPage((v) => v + 1)}
            >
              {t("quoteNext")}
            </Button>
          </div>
        )}
      </div>
      <p className="text-sm text-muted">{t("quoteSentHint")}</p>
      <Dialog
        open={compose}
        onClose={() => {
          if (!busy) setCompose(false);
        }}
        title={`${t(quote ? "quoteMailTitle" : "invoiceMailTitle")} · ${doc.number}`}
        description={t(
          share ? "shareMailHint" : quote ? "quoteMailHint" : "invoiceMailHint",
        )}
      >
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            void send();
          }}
        >
          {quote && (
            <>
              {" "}
              <Select
                label={t("shareMethod")}
                value={share ? "link" : "attachment"}
                disabled={busy}
                onChange={method}
                options={[
                  { value: "attachment", label: t("shareAttachmentMethod") },
                  {
                    value: "link",
                    label: t("shareLinkMethod"),
                    disabled: !available.length,
                  },
                ]}
              />
              {!available.length && (
                <p className="text-sm text-muted">{t("shareNoActive")}</p>
              )}
              {share && available.length > 1 && (
                <Select
                  label={t("shareSelect")}
                  value={share.id}
                  disabled={busy}
                  onChange={(id) =>
                    setShare(available.find((v) => v.id === id) ?? null)
                  }
                  options={available.map((v) => ({
                    value: v.id,
                    label:
                      date(v.created_at) +
                      " · " +
                      t("shareExpires") +
                      " " +
                      date(v.expires_at),
                  }))}
                />
              )}
            </>
          )}
          <dl className="text-sm">
            <div>
              <dt className="inline text-muted">{t("quoteFrom")}: </dt>
              <dd className="inline">{history?.sender}</dd>
            </div>
            {data.supplier.email && (
              <div>
                <dt className="inline text-muted">{t("quoteReplyTo")}: </dt>
                <dd className="inline">{data.supplier.email}</dd>
              </div>
            )}
          </dl>
          <Field
            label={t("quoteTo")}
            type="email"
            required
            maxLength={254}
            value={recipient}
            disabled={busy}
            onChange={(e) => setRecipient(e.target.value)}
          />
          <Field
            label={t("quoteSubject")}
            required
            maxLength={200}
            value={subject}
            disabled={busy}
            onChange={(e) => setSubject(e.target.value)}
          />
          <TextareaField
            label={t("quoteBody")}
            required
            rows={8}
            maxLength={20000}
            value={body}
            disabled={busy}
            onChange={(e) => setBody(e.target.value)}
          />
          <DocumentAttachments
            key={String(compose)}
            company={doc.company_id}
            kind={quote ? "quote" : "invoice"}
            id={doc.id}
            selected={attachmentIds}
            onSelect={setAttachmentIds}
            disabled={busy}
          />
          <p className="break-all text-sm">
            {share ? t("shareLinkMethod") : t("quoteAttachment")}:{" "}
            {share ? share.url : doc.number + ".pdf"} · {t("quoteRevision")}{" "}
            {doc.revision}
          </p>
          {error && (
            <Alert tone="danger" role="alert">
              {t(error)}
            </Alert>
          )}
          <div className="flex justify-end gap-control">
            <Button
              type="button"
              variant="secondary"
              disabled={busy}
              onClick={() => setCompose(false)}
            >
              {t("quoteCancel")}
            </Button>
            <Button type="submit" disabled={busy}>
              {t(busy ? "quoteSending" : "quoteSend")}
            </Button>
          </div>
        </form>
      </Dialog>
      <Dialog
        open={!!detail}
        onClose={() => setDetail(null)}
        title={t("quoteMailDetail")}
        description={detail?.subject ?? ""}
      >
        {detail && (
          <div className="space-y-field">
            {error && <Alert tone="danger">{t(error)}</Alert>}
            {detail.error_code && failureReasons[detail.error_code] && (
              <Alert
                tone={detail.status === "uncertain" ? "warning" : "danger"}
              >
                {t(failureReasons[detail.error_code])}
              </Alert>
            )}
            <dl className="space-y-inline text-sm">
              {[
                [t("quoteFrom"), detail.sender],
                [t("quoteReplyTo"), detail.reply_to || "—"],
                [t("quoteTo"), detail.recipient],
                [t("quoteMailDate"), date(detail.created_at)],
                [
                  t("quoteMailFinished"),
                  detail.finished_at ? date(detail.finished_at) : "—",
                ],
                [t("quoteMailStatus"), t(statuses[detail.status])],
                [t("quoteRevision"), String(detail.revision)],
                [
                  t("shareMethod"),
                  t(
                    detail.share_id
                      ? "shareMethodLink"
                      : "shareMethodAttachment",
                  ),
                ],
              ].map(([k, v]) => (
                <div key={k}>
                  <dt className="text-muted">{k}</dt>
                  <dd className="break-words">{v}</dd>
                </div>
              ))}
            </dl>
            <p className="whitespace-pre-wrap break-words text-sm">
              {detail.body}
            </p>
            {detail.attachments?.length > 0 && (
              <section className="space-y-inline">
                <Heading level={3} size="card">
                  {t("attachmentsHistory")}
                </Heading>
                {detail.attachments.map((file) => (
                  <Button
                    key={file.id}
                    variant="secondary"
                    className="max-w-full break-all"
                    disabled={busy}
                    onClick={() => {
                      setBusy(true);
                      void saveAttachmentFile(
                        doc.company_id,
                        quote ? "quote" : "invoice",
                        doc.id,
                        file,
                        detail.id,
                      )
                        .catch(() => setError("attachmentsError"))
                        .finally(() => setBusy(false));
                    }}
                  >
                    {file.filename}
                  </Button>
                ))}
              </section>
            )}
            <Button
              disabled={busy}
              variant="secondary"
              onClick={() => void pdf(false, detail.id)}
            >
              {t(quote ? "quotePdf" : "invoiceDownload")}
            </Button>
          </div>
        )}
      </Dialog>
      <Dialog
        open={!!preview}
        onClose={() => setPreview(null)}
        title={t(quote ? "quotePrint" : "invoicePrint")}
        description={t("quotePrintHint")}
      >
        {preview && (
          <div className="space-y-field">
            <iframe
              title={t(quote ? "quotePrint" : "invoicePrint")}
              src={preview}
              className="h-pdf-preview w-full rounded-control border border-border"
            />
            <a
              className="ui-button ui-button-secondary ui-button-md"
              href={preview}
              target="_blank"
              rel="noreferrer"
            >
              {t("quoteOpenPdf")}
            </a>
          </div>
        )}
      </Dialog>
    </section>
  );
}
