import { useEffect, useRef, useState } from "react";
import {
  documentStyle,
  type StyleSnapshot,
  type DocumentStyle,
  type StyleCommand,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
import { useWorkspace } from "./useCompanyWorkspace";
export function useDocumentStyle(company: string) {
  const { setPageDirty, onExpired } = useWorkspace();
  const [saved, setSaved] = useState<StyleSnapshot>();
  const [data, setData] = useState<DocumentStyle>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const pending = useRef<StyleCommand | undefined>(undefined);
  const dirty =
    !!data && !!saved && JSON.stringify(data) !== JSON.stringify(saved.data);
  useEffect(() => {
    const abort = new AbortController();
    setData(undefined);
    setSaved(undefined);
    pending.current = undefined;
    documentStyle(apiUrl, company, undefined, abort.signal)
      .then((v) => {
        setData(v.data);
        setSaved(v);
      })
      .catch((e: unknown) => {
        if (abort.signal.aborted) return;
        if ((e as { status?: number }).status === 401) onExpired();
        else setError(true);
      });
    return () => abort.abort();
  }, [company, onExpired]);
  useEffect(() => {
    setPageDirty(dirty);
    return () => setPageDirty(false);
  }, [dirty, setPageDirty]);
  async function save() {
    if (!data || !saved || busy) return;
    setBusy(true);
    setError(false);
    const old = pending.current;
    const command =
      old && JSON.stringify(old.data) === JSON.stringify(data)
        ? old
        : { request_id: crypto.randomUUID(), revision: saved.revision, data };
    pending.current = command;
    try {
      const v = await documentStyle(apiUrl, company, command);
      setSaved(v);
      setData(v.data);
      pending.current = undefined;
    } catch (e: unknown) {
      if ((e as { status?: number }).status === 401) onExpired();
      else setError(true);
    } finally {
      setBusy(false);
    }
  }
  async function image(file: File, key: "logo_base64" | "signature_base64") {
    if (!data) return;
    if (
      file.size > 100000 ||
      !["image/png", "image/jpeg"].includes(file.type)
    ) {
      setError(true);
      return;
    }
    const bytes = new Uint8Array(await file.arrayBuffer());
    let value = "";
    for (const b of bytes) value += String.fromCharCode(b);
    setData({ ...data, [key]: btoa(value) });
  }
  return { data, setData, busy, error, dirty, save, image };
}
