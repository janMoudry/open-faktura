import { documentImageDimensions } from "../../utils/documentImage";
import { useEffect, useRef, useState } from "react";
import { Alert, Button, Heading } from "@open-faktura/ui";
import { inboxFileUrl } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
const LIMIT = 2097152;
export function InboxOriginal({
  companyId,
  id,
}: {
  companyId: string;
  id: string;
}) {
  const { t } = useAppTranslation();
  const canvas = useRef<HTMLCanvasElement>(null);
  const [source, setSource] = useState<{
      bytes: Uint8Array;
      kind: "pdf" | "image" | "text";
    } | null>(null),
    [page, setPage] = useState(1),
    [pages, setPages] = useState(1),
    [busy, setBusy] = useState(true),
    [error, setError] = useState(false);
  useEffect(() => {
    const abort = new AbortController();
    setSource(null);
    setPage(1);
    setPages(1);
    setError(false);
    setBusy(true);
    (async () => {
      const response = await fetch(inboxFileUrl(apiUrl, companyId, id), {
        credentials: "include",
        signal: abort.signal,
      });
      if (!response.ok || !response.body) throw new Error("original");
      const reader = response.body.getReader();
      const chunks: Uint8Array[] = [];
      let size = 0;
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        size += value.length;
        if (size > LIMIT) {
          await reader.cancel();
          throw new Error("limit");
        }
        chunks.push(value);
      }
      const bytes = new Uint8Array(size);
      let offset = 0;
      for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.length;
      }
      const prefix = String.fromCharCode(...bytes.subarray(0, 8));
      const kind = prefix.startsWith("%PDF-")
        ? "pdf"
        : prefix.startsWith("\x89PNG\r\n\x1a\n") ||
            (bytes[0] === 255 && bytes[1] === 216 && bytes[2] === 255)
          ? "image"
          : "text";
      if (!abort.signal.aborted) {
        setSource({ bytes, kind });
        if (kind === "text") setBusy(false);
      }
    })().catch(() => {
      if (!abort.signal.aborted) {
        setError(true);
        setBusy(false);
      }
    });
    return () => abort.abort();
  }, [companyId, id]);
  useEffect(() => {
    if (!source || source.kind === "text") return;
    let active = true;
    let destroy: (() => void) | undefined;
    setBusy(true);
    setError(false);
    (async () => {
      if (source.kind === "image") {
        documentImageDimensions(source.bytes);
        const bitmap = await createImageBitmap(
          new Blob([source.bytes.slice().buffer]),
        );
        try {
          if (!active) return;
          if (
            bitmap.width > 4096 ||
            bitmap.height > 4096 ||
            bitmap.width * bitmap.height > 16_000_000
          )
            throw new Error("dimensions");
          const target = canvas.current;
          if (!target) return;
          const scale = Math.min(1, 1200 / bitmap.width, 1600 / bitmap.height);
          target.width = Math.max(1, Math.round(bitmap.width * scale));
          target.height = Math.max(1, Math.round(bitmap.height * scale));
          target
            .getContext("2d")
            ?.drawImage(bitmap, 0, 0, target.width, target.height);
        } finally {
          bitmap.close();
        }
      } else {
        const pdfjs = await import("pdfjs-dist");
        const worker = await import("pdfjs-dist/build/pdf.worker.min.mjs?url");
        if (!active) return;
        pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
        const task = pdfjs.getDocument({
          data: source.bytes.slice(),
          enableXfa: false,
          useWorkerFetch: false,
          useWasm: false,
          disableFontFace: true,
          useSystemFonts: false,
          disableAutoFetch: true,
          disableStream: true,
          maxImageSize: 16_000_000,
          canvasMaxAreaInBytes: 16_000_000,
          verbosity: 0,
        });
        destroy = () => {
          void task.destroy();
        };
        const pdf = await task.promise;
        if (!active) return;
        if (pdf.numPages > 50) throw new Error("pages");
        setPages(pdf.numPages);
        const pdfPage = await pdf.getPage(page);
        if (!active) return;
        const natural = pdfPage.getViewport({ scale: 1 });
        const scale = Math.min(
          1.5,
          1200 / natural.width,
          1600 / natural.height,
        );
        const viewport = pdfPage.getViewport({ scale });
        const target = canvas.current;
        if (
          !target ||
          !Number.isFinite(viewport.width) ||
          !Number.isFinite(viewport.height)
        )
          throw new Error("viewport");
        target.width = Math.max(1, Math.floor(viewport.width));
        target.height = Math.max(1, Math.floor(viewport.height));
        const context = target.getContext("2d");
        if (!context) throw new Error("canvas");
        await pdfPage.render({
          canvas: target,
          canvasContext: context,
          viewport,
          annotationMode: pdfjs.AnnotationMode.DISABLE,
          background: "white",
        }).promise;
      }
      if (active) setBusy(false);
    })()
      .catch(() => {
        if (active) {
          setError(true);
          setBusy(false);
        }
      })
      .finally(() => destroy?.());
    return () => {
      active = false;
      destroy?.();
    };
  }, [source, page]);
  return (
    <aside
      className="workspace-panel min-w-0 space-y-field p-field lg:sticky lg:top-field"
      aria-label={t("inboxOriginal")}
    >
      <Heading level={2}>{t("inboxOriginal")}</Heading>
      <a
        className="text-sm underline"
        href={inboxFileUrl(apiUrl, companyId, id)}
      >
        {t("inboxOriginalDownload")}
      </a>
      {busy && <p role="status">{t("cashLoading")}</p>}
      {error && <Alert tone="danger">{t("inboxPreviewFailed")}</Alert>}
      <canvas
        ref={canvas}
        role="img"
        aria-label={t("inboxOriginalPage", { page })}
        className={
          source && source.kind !== "text" && !error
            ? "h-auto w-full"
            : "hidden"
        }
      />
      {source?.kind === "text" && (
        <pre className="max-h-[70vh] overflow-auto whitespace-pre-wrap break-all text-xs">
          {new TextDecoder().decode(source.bytes.subarray(0, 200000))}
        </pre>
      )}
      {source?.kind === "pdf" && pages > 1 && (
        <div className="flex flex-wrap items-center gap-control">
          <Button
            variant="secondary"
            disabled={busy || page <= 1}
            onClick={() => setPage((v) => v - 1)}
          >
            {t("retailPrevious")}
          </Button>
          <span>{t("inboxPreviewPageCount", { page, pages })}</span>
          <Button
            variant="secondary"
            disabled={busy || page >= pages}
            onClick={() => setPage((v) => v + 1)}
          >
            {t("retailNext")}
          </Button>
        </div>
      )}
    </aside>
  );
}
