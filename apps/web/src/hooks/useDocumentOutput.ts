import { useState } from "react";
import {
  documentOutput,
  type DocumentOutputRequest,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
export function useDocumentOutput(company: string) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  async function download(request: DocumentOutputRequest) {
    if (busy) return;
    setBusy(true);
    setError(false);
    try {
      const blob = await documentOutput(apiUrl, company, request);
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${request.kind}.${request.kind === "rtf" ? "rtf" : "pdf"}`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  return { busy, error, download };
}
