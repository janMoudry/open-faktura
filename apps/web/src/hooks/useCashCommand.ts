import { cashError } from "../utils/cash";
import { useRef, useState } from "react";
import {
  cashExecute,
  SalesFailure,
  type CashCommand,
} from "@open-faktura/api-client";
import { useWorkspace } from "./useCompanyWorkspace";
import { apiUrl } from "../services/status";
import type { AppKey } from "../locales";
export function useCashCommand(companyId: string) {
  const { onExpired } = useWorkspace();
  const [busy, setBusy] = useState(false),
    [error, setError] = useState<AppKey | "">("");
  const pending = useRef<{ input: string; id: string } | null>(null);
  async function execute(command: CashCommand) {
    const input = JSON.stringify({
      ...command,
      request_id: undefined,
      companyId,
    });
    if (pending.current?.input !== input)
      pending.current = { input, id: crypto.randomUUID() };
    setBusy(true);
    setError("");
    try {
      const r = await cashExecute(apiUrl, companyId, {
        ...command,
        request_id: pending.current.id,
      });
      pending.current = null;
      return r;
    } catch (e) {
      if (e instanceof SalesFailure && e.status === 401) onExpired();
      setError(cashError(e));
      return null;
    } finally {
      setBusy(false);
    }
  }
  return { execute, busy, error, setError };
}
