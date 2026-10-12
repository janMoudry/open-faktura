import { useEffect, useRef, useState } from "react";
import { Alert, Button, Checkbox, Heading } from "@open-faktura/ui";
import {
  listAttachments,
  uploadAttachment,
  removeAttachment,
  downloadAttachment,
  SalesFailure,
  type DocumentAttachment,
  type AttachmentKind,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
export async function saveAttachmentFile(
  c: string,
  k: AttachmentKind,
  d: string,
  f: DocumentAttachment,
  delivery?: string,
) {
  const blob = await downloadAttachment(apiUrl, c, k, d, f.id, delivery);
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = f.filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
export function DocumentAttachments({
  company,
  kind,
  id,
  editable = false,
  selected,
  onSelect,
  disabled = false,
}: {
  company: string;
  kind: AttachmentKind;
  id?: string;
  editable?: boolean;
  selected?: string[];
  onSelect?: (ids: string[]) => void;
  disabled?: boolean;
}) {
  const { t, language } = useAppTranslation();
  const { onExpired } = useWorkspace();
  const [files, setFiles] = useState<DocumentAttachment[]>([]);
  const [loading, setLoading] = useState(!!id);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const [reload, setReload] = useState(0);
  const [pending, setPending] = useState<{ file: File; key: string }[]>([]);
  const input = useRef<HTMLInputElement>(null);
  const size = (v: number) =>
    v < 1024
      ? `${v} B`
      : new Intl.NumberFormat(language === "cs" ? "cs-CZ" : "en-GB", {
          maximumFractionDigits: 1,
        }).format(v / 1024) + " KB";
  function fail(e: unknown) {
    if (e instanceof SalesFailure && e.status === 401) {
      onExpired();
      return;
    }
    const field = e instanceof SalesFailure ? e.fields[0] : "";
    setError(
      field === "file_size"
        ? "attachmentsSizeError"
        : field === "file_type"
          ? "attachmentsTypeError"
          : field === "file_count"
            ? "attachmentsCountError"
            : field === "storage_limit"
              ? "attachmentsStorageError"
              : "attachmentsError",
    );
  }
  useEffect(() => {
    if (!id) return;
    let active = true;
    setLoading(true);
    listAttachments(apiUrl, company, kind, id)
      .then((v) => {
        if (active) setFiles(v);
      })
      .catch((e) => {
        if (active) fail(e);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [company, kind, id, reload]);
  async function upload(queue: { file: File; key: string }[]) {
    if (!id || busy || disabled) return;
    setBusy(true);
    setError("");
    let rest = [...queue];
    setPending(rest);
    try {
      while (rest.length) {
        const first = rest[0];
        await uploadAttachment(
          apiUrl,
          company,
          kind,
          id,
          first.file,
          first.key,
        );
        rest = rest.slice(1);
        setPending(rest);
      }
    } catch (e) {
      fail(e);
    } finally {
      setReload((v) => v + 1);
      setBusy(false);
    }
  }
  function choose(list: File[]) {
    if (list.length + files.length > 5) {
      setError("attachmentsCountError");
      return;
    }
    void upload(list.map((file) => ({ file, key: crypto.randomUUID() })));
  }
  if (!id)
    return (
      <p className="p-field text-sm text-muted">{t("attachmentsSaveFirst")}</p>
    );
  return (
    <section
      className="space-y-field bg-surface p-field"
      aria-label={t("invoiceAttachmentsTab")}
      onDragOver={(e) => {
        if (editable && !busy) {
          e.preventDefault();
        }
      }}
      onDrop={(e) => {
        e.preventDefault();
        if (editable && !busy && !disabled)
          choose(Array.from(e.dataTransfer.files));
      }}
    >
      <div className="flex flex-wrap items-center justify-between gap-control">
        <Heading level={3} size="card">
          {t(onSelect ? "attachmentsEmailTitle" : "invoiceAttachmentsTab")}
        </Heading>
        {editable && (
          <>
            <input
              ref={input}
              type="file"
              multiple
              hidden
              accept=".pdf,.png,.jpg,.jpeg,.zip,.rar,.xlsx,.docx,.odt,.ods,.txt"
              onChange={(e) => {
                choose(Array.from(e.target.files ?? []));
                e.target.value = "";
              }}
            />
            <Button
              type="button"
              variant="secondary"
              disabled={busy || disabled || files.length >= 5}
              onClick={() => input.current?.click()}
            >
              {t(busy ? "attachmentsUploading" : "attachmentsAdd")}
            </Button>
          </>
        )}
      </div>
      <p className="text-sm text-muted">
        {t(onSelect ? "attachmentsEmailHint" : "attachmentsHint")}
      </p>
      {error && (
        <Alert role="alert" tone="danger">
          {t(error)}{" "}
          <Button
            type="button"
            variant="ghost"
            disabled={busy || disabled}
            onClick={() => {
              setError("");
              if (pending.length) void upload(pending);
              else setReload((v) => v + 1);
            }}
          >
            {t("attachmentsRetry")}
          </Button>
        </Alert>
      )}
      {loading ? (
        <p role="status">{t("attachmentsLoading")}</p>
      ) : files.length === 0 ? (
        <p className="text-sm text-muted">{t("attachmentsEmpty")}</p>
      ) : (
        <ul className="divide-y divide-border border-y border-border">
          {files.map((f) => (
            <li
              key={f.id}
              className="flex flex-wrap items-center justify-between gap-control py-control"
            >
              <div className="min-w-0 flex-1">
                {onSelect ? (
                  <Checkbox
                    label={`${f.filename} (${size(f.size)})`}
                    checked={selected?.includes(f.id) ?? false}
                    disabled={disabled || busy}
                    onChange={(e) =>
                      onSelect(
                        e.target.checked
                          ? [...(selected ?? []), f.id]
                          : (selected ?? []).filter((v) => v !== f.id),
                      )
                    }
                  />
                ) : (
                  <>
                    <p className="break-all text-sm font-medium">
                      {f.filename}
                    </p>
                    <p className="text-xs text-muted">{size(f.size)}</p>
                  </>
                )}
              </div>
              <div className="flex gap-inline">
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  disabled={busy || disabled}
                  onClick={() => {
                    setBusy(true);
                    setError("");
                    void saveAttachmentFile(company, kind, id, f)
                      .catch(fail)
                      .finally(() => setBusy(false));
                  }}
                >
                  {t("attachmentsDownload")}
                </Button>
                {editable && (
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    disabled={busy || disabled}
                    onClick={() => {
                      if (
                        !window.confirm(
                          t("attachmentsDeleteConfirm", { name: f.filename }),
                        )
                      )
                        return;
                      setBusy(true);
                      setError("");
                      void removeAttachment(apiUrl, company, kind, id, f.id)
                        .then(() => setReload((v) => v + 1))
                        .catch(fail)
                        .finally(() => setBusy(false));
                    }}
                  >
                    {t("attachmentsRemove")}
                  </Button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
