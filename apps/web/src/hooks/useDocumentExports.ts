import { useEffect, useRef, useState } from "react";
import {
  documentExportHistory,
  documentExportCreate,
  documentExportUnlock,
  documentExportFile,
  type ExportVersion,
  type ExportHistory,
  type DocumentExportRequest,
  type DocumentExportUnlock,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
export function useDocumentExports(company: string, target: ExportVersion) {
  const [history, setHistory] = useState<ExportHistory>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const pending = useRef<DocumentExportRequest | undefined>(undefined);
  const pendingUnlock = useRef<DocumentExportUnlock | undefined>(undefined);
  const scope = useRef("");
  const key = `${company}:${target.kind}:${target.id}:${target.revision}`;
  scope.current = key;
  useEffect(() => {
    const abort = new AbortController();
    setHistory(undefined);
    setError(false);
    pending.current = undefined;
    pendingUnlock.current = undefined;
    documentExportHistory(apiUrl, company, target, abort.signal)
      .then(setHistory)
      .catch(() => {
        if (!abort.signal.aborted) setError(true);
      });
    return () => abort.abort();
  }, [company, target.kind, target.id, target.revision]);
  function save(blob: Blob, id = "document-export") {
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${id}.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  async function create(lock: boolean) {
    if (busy) return false;
    setBusy(true);
    setError(false);
    const current = key;
    const old = pending.current;
    const request =
      old && old.lock === lock
        ? old
        : { request_id: crypto.randomUUID(), targets: [target], lock };
    pending.current = request;
    try {
      const blob = await documentExportCreate(apiUrl, company, request);
      if (scope.current !== current) return false;
      save(blob);
      setHistory(await documentExportHistory(apiUrl, company, target));
      pending.current = undefined;
      return true;
    } catch {
      if (scope.current === current) setError(true);
      return false;
    } finally {
      if (scope.current === current) setBusy(false);
    }
  }
  async function unlock(reason: string) {
    if (busy || !history?.locked_by_export) return false;
    setBusy(true);
    setError(false);
    const current = key;
    const old = pendingUnlock.current;
    const command =
      old && old.reason === reason
        ? old
        : {
            request_id: crypto.randomUUID(),
            target: { kind: target.kind, id: target.id },
            export_id: history.locked_by_export,
            reason,
          };
    pendingUnlock.current = command;
    try {
      const result = await documentExportUnlock(apiUrl, company, command);
      if (scope.current !== current) return false;
      setHistory(result);
      pendingUnlock.current = undefined;
      return true;
    } catch {
      if (scope.current === current) setError(true);
      return false;
    } finally {
      if (scope.current === current) setBusy(false);
    }
  }
  async function download(id: string) {
    if (busy) return;
    setBusy(true);
    setError(false);
    try {
      save(
        await documentExportFile(apiUrl, company, id),
        `document-export-${id}`,
      );
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  return { history, busy, error, create, unlock, download };
}
