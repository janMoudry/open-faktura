import { InboxMailbox } from "../components/organisms/InboxMailbox";
import { InboxAdapters } from "../components/organisms/InboxAdapters";
import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import {
  Alert,
  Dialog,
  Badge,
  Button,
  Field,
  Heading,
  Select,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  inboxList,
  inboxHistory,
  type InboxAudit,
  listReceived,
  type ReceivedInvoice,
  inboxUpload,
  inboxExecute,
  inboxFileUrl,
  type InboxDocument,
  type InboxPage as Page,
  SalesFailure,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation } from "../locales";
import { apiUrl } from "../services/status";
export function InboxPage() {
  const { companyId = "" } = useParams(),
    { company, onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [data, setData] = useState<Page | null>(null),
    [query, setQuery] = useState(""),
    [archived, setArchived] = useState(false),
    [page, setPage] = useState(1),
    [attempt, setAttempt] = useState(0),
    [busy, setBusy] = useState(false),
    [dragging, setDragging] = useState(false),
    [error, setError] = useState(false),
    [history, setHistory] = useState<InboxAudit[] | null>(null),
    [attaching, setAttaching] = useState<InboxDocument | null>(null),
    [candidates, setCandidates] = useState<ReceivedInvoice[]>([]),
    [targetQuery, setTargetQuery] = useState(""),
    [targetKind, setTargetKind] = useState<"invoice" | "receipt">("invoice");
  useEffect(() => {
    if (!attaching) return;
    const abort = new AbortController();
    setCandidates([]);
    listReceived(
      apiUrl,
      companyId,
      targetQuery,
      "",
      1,
      abort.signal,
      targetKind,
    )
      .then((v) => setCandidates(v.items.filter((d) => !d.cancelled)))
      .catch(() => {
        if (!abort.signal.aborted) setError(true);
      });
    return () => abort.abort();
  }, [attaching, companyId, targetQuery, targetKind]);
  const write = company?.id === companyId && company.role !== "viewer";
  useEffect(() => {
    const c = new AbortController();
    setError(false);
    inboxList(apiUrl, companyId, query, archived, page, c.signal)
      .then(setData)
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, query, archived, page, attempt]);
  async function run(fn: () => Promise<unknown>) {
    setBusy(true);
    setError(false);
    try {
      await fn();
      setAttempt((v) => v + 1);
    } catch (e) {
      setError(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  async function upload(files: File[]) {
    await run(async () => {
      for (const f of files) {
        if (f.size > 2097152) throw new Error("file size");
        const bytes = new Uint8Array(await f.arrayBuffer());
        let binary = "";
        for (let i = 0; i < bytes.length; i += 8192)
          binary += String.fromCharCode(...bytes.subarray(i, i + 8192));
        await inboxUpload(apiUrl, companyId, {
          request_id: crypto.randomUUID(),
          filename: f.name,
          base64: btoa(binary),
        });
      }
    });
  }
  function archive(d: InboxDocument) {
    return run(() =>
      inboxExecute(apiUrl, companyId, {
        request_id: crypto.randomUUID(),
        id: d.id,
        revision: d.revision,
        action: "archive",
        archived: !d.archived,
      }),
    );
  }
  return (
    <div className="space-y-group min-w-0">
      <Heading level={1} size="page">
        {t("inboxTitle")}
      </Heading>
      <p className="text-muted">{t("inboxIntro")}</p>
      {error && (
        <Alert tone="danger">
          {t("inboxFailed")}{" "}
          <Button onClick={() => setAttempt((v) => v + 1)}>
            {t("cashRetry")}
          </Button>
        </Alert>
      )}
      <section className="workspace-panel min-w-0">
        <div className="flex flex-wrap items-end gap-field p-field">
          {write && (
            <div
              role="group"
              aria-label={t("inboxDropzone")}
              className={
                dragging
                  ? "rounded-control border-2 border-primary bg-primary-soft p-field"
                  : "rounded-control border border-dashed border-border p-field"
              }
              onDragOver={(e) => {
                if (
                  !busy &&
                  Array.from(e.dataTransfer.types).includes("Files")
                ) {
                  e.preventDefault();
                  setDragging(true);
                }
              }}
              onDragLeave={(e) => {
                if (!e.currentTarget.contains(e.relatedTarget as Node | null))
                  setDragging(false);
              }}
              onDrop={(e) => {
                e.preventDefault();
                setDragging(false);
                if (!busy) void upload(Array.from(e.dataTransfer.files));
              }}
            >
              <p className="mb-control text-sm text-muted">
                {t("inboxDropzone")}
              </p>
              <Field
                type="file"
                label={t("inboxUpload")}
                accept=".pdf,.png,.jpg,.jpeg,.isdoc"
                multiple
                disabled={busy}
                onChange={(e) => {
                  const files = Array.from(e.target.files ?? []);
                  e.target.value = "";
                  void upload(files);
                }}
              />
            </div>
          )}
          <Field
            label={t("inboxSearch")}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(1);
            }}
          />
          <Select
            label={t("terminalStatus")}
            value={String(archived)}
            onChange={(v) => {
              setArchived(v === "true");
              setPage(1);
            }}
            options={[
              { value: "false", label: t("retailActive") },
              { value: "true", label: t("terminalArchived") },
            ]}
          />
        </div>
        {busy && (
          <p className="p-field" role="status">
            {t("cashLoading")}
          </p>
        )}
        <div className="overflow-x-auto">
          <Table className="whitespace-nowrap">
            <TableHeader>
              <TableRow>
                <TableHead>{t("inboxFile")}</TableHead>
                <TableHead>{t("inboxDate")}</TableHead>
                <TableHead>{t("terminalStatus")}</TableHead>
                <TableHead>{t("inboxActions")}</TableHead>
              </TableRow>
            </TableHeader>
            <tbody>
              {data?.items.map((d) => (
                <TableRow key={d.id}>
                  <TableCell>
                    <a
                      className="underline underline-offset-4"
                      href={inboxFileUrl(apiUrl, companyId, d.id)}
                    >
                      {d.filename}
                    </a>
                  </TableCell>
                  <TableCell>
                    {new Date(d.created_at).toLocaleDateString(language)}
                  </TableCell>
                  <TableCell>
                    <Badge>
                      {t(d.received_id ? "inboxProcessed" : "inboxPending")}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-control">
                      {d.received_id ? (
                        <Link
                          className="ui-button ui-button-secondary ui-button-sm"
                          to={`/companies/${companyId}/${d.received_kind === "receipt" ? "receipts" : "received-invoices"}/${d.received_id}`}
                        >
                          {t("inboxDocument")}
                        </Link>
                      ) : (
                        write &&
                        !d.archived && (
                          <Link
                            className="ui-button ui-button-secondary ui-button-sm"
                            to={`/companies/${companyId}/received-invoices/new?inbox=${d.id}&revision=${d.revision}`}
                          >
                            {t("inboxCreate")}
                          </Link>
                        )
                      )}
                      {write && !d.archived && !d.received_id && (
                        <Button
                          variant="secondary"
                          disabled={busy}
                          onClick={() => setAttaching(d)}
                        >
                          {t("inboxAttach")}
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        onClick={() =>
                          run(async () =>
                            setHistory(
                              await inboxHistory(apiUrl, companyId, d.id),
                            ),
                          )
                        }
                      >
                        {t("inboxHistory")}
                      </Button>
                      {write && (
                        <Button
                          variant="ghost"
                          disabled={busy}
                          onClick={() => archive(d)}
                        >
                          {t(
                            d.archived ? "terminalRestore" : "terminalArchive",
                          )}
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </tbody>
          </Table>
        </div>
        {data?.items.length === 0 && (
          <p className="p-field">{t("inboxEmpty")}</p>
        )}
        <div className="flex justify-end gap-control p-field">
          <Button
            variant="secondary"
            disabled={page === 1 || busy}
            onClick={() => setPage((v) => v - 1)}
          >
            {t("retailPrevious")}
          </Button>
          <Button
            variant="secondary"
            disabled={!data || page * 25 >= data.total || busy}
            onClick={() => setPage((v) => v + 1)}
          >
            {t("retailNext")}
          </Button>
        </div>
      </section>
      <InboxMailbox
        key={`mailbox:${companyId}`}
        companyId={companyId}
        write={write}
        onImported={() => setAttempt((v) => v + 1)}
      />
      <InboxAdapters key={companyId} companyId={companyId} write={write} />
      <Dialog
        open={!!attaching}
        onClose={() => {
          if (!busy) setAttaching(null);
        }}
        title={t("inboxAttach")}
        description={t("inboxFindDocument")}
      >
        <div className="space-y-field">
          <Field
            label={t("inboxFindDocument")}
            value={targetQuery}
            onChange={(e) => setTargetQuery(e.target.value)}
          />
          <Select
            label={t("inboxDocumentType")}
            value={targetKind}
            onChange={(v) => setTargetKind(v as "invoice" | "receipt")}
            options={[
              { value: "invoice", label: t("receivedTitle") },
              { value: "receipt", label: t("receiptsTitle") },
            ]}
          />
          {candidates.map((d) => (
            <Button
              key={d.id}
              variant="secondary"
              disabled={busy}
              onClick={() =>
                run(async () => {
                  if (!attaching) return;
                  await inboxExecute(apiUrl, companyId, {
                    request_id: crypto.randomUUID(),
                    id: attaching.id,
                    revision: attaching.revision,
                    action: "attach",
                    received_id: d.id,
                  });
                  setAttaching(null);
                })
              }
            >
              {d.number} · {d.data.supplier.name}
            </Button>
          ))}
          {!candidates.length && <p>{t("inboxEmpty")}</p>}
        </div>
      </Dialog>
      <Dialog
        open={history !== null}
        onClose={() => setHistory(null)}
        title={t("inboxHistory")}
        description={t("inboxHistoryDescription")}
      >
        <div className="space-y-field">
          {history?.map((h) => (
            <p key={h.id}>
              {new Date(h.created_at).toLocaleString(language)} ·{" "}
              {t("retailRevision")} {h.revision} ·{" "}
              {t(
                h.action === "mail"
                  ? "inboxMailReceived"
                  : h.action === "upload"
                    ? "inboxUpload"
                    : h.action === "process"
                      ? "inboxProcessed"
                      : "inboxArchiveEvent",
              )}
            </p>
          ))}
        </div>
      </Dialog>
    </div>
  );
}
