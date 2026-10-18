import { useEffect, useRef, useState } from "react";
import {
  documentTags,
  type DocumentTag,
  type TagTarget,
  type TagCommand,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
export function useDocumentTags(company: string, target?: TagTarget) {
  const [all, setAll] = useState<DocumentTag[]>([]);
  const [selected, setSelected] = useState<DocumentTag[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const key = `${company}:${target?.kind ?? ""}:${target?.id ?? ""}`;
  const scope = useRef(key);
  scope.current = key;
  const pending = useRef<TagCommand | undefined>(undefined);
  useEffect(() => {
    const abort = new AbortController();
    setAll([]);
    setSelected([]);
    setError(false);
    pending.current = undefined;
    Promise.all([
      documentTags(apiUrl, company, undefined, abort.signal),
      target
        ? documentTags(apiUrl, company, target, abort.signal)
        : Promise.resolve([]),
    ])
      .then(([a, s]) => {
        setAll(a);
        setSelected(s);
      })
      .catch(() => {
        if (!abort.signal.aborted) setError(true);
      });
    return () => abort.abort();
  }, [company, target?.kind, target?.id]);
  async function execute(action: TagCommand) {
    if (busy) return false;
    setBusy(true);
    setError(false);
    const captured = key;
    const old = pending.current;
    const same =
      old &&
      JSON.stringify({ ...old, request_id: undefined }) ===
        JSON.stringify({ ...action, request_id: undefined });
    const command = same ? old : action;
    pending.current = command;
    try {
      const a = await documentTags(apiUrl, company, command);
      const s = target ? await documentTags(apiUrl, company, target) : [];
      if (scope.current !== captured) return false;
      pending.current = undefined;
      setAll(a);
      setSelected(s);
      return true;
    } catch {
      if (scope.current === captured) setError(true);
      return false;
    } finally {
      if (scope.current === captured) setBusy(false);
    }
  }
  return { all, selected, busy, error, execute };
}
