import { useEffect, useRef, useState } from "react";
import {
  companyAdministration,
  companyTrash,
  companyArchive,
  type AdministrationSnapshot,
  type AdministrationAction,
} from "@open-faktura/api-client";
import { apiUrl } from "../services/status";
export function useCompanyAdministration(
  company: string | undefined,
  onExpired: () => void,
) {
  const [data, setData] = useState<AdministrationSnapshot>();
  const [trash, setTrash] = useState<AdministrationSnapshot[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const scope = useRef(company);
  scope.current = company;
  const pending = useRef<
    | {
        company: string;
        command: AdministrationAction & {
          request_id: string;
          revision: number;
        };
      }
    | undefined
  >(undefined);
  useEffect(() => {
    const abort = new AbortController();
    setData(undefined);
    setTrash([]);
    setError(false);
    pending.current = undefined;
    const load = company
      ? companyAdministration(apiUrl, company, undefined, abort.signal).then(
          setData,
        )
      : companyTrash(apiUrl, abort.signal).then(setTrash);
    load.catch((e: unknown) => {
      if (abort.signal.aborted) return;
      if ((e as { status?: number }).status === 401) onExpired();
      else setError(true);
    });
    return () => abort.abort();
  }, [company, onExpired]);
  async function execute(
    target: AdministrationSnapshot,
    action: AdministrationAction,
  ) {
    if (busy) return false;
    setBusy(true);
    setError(false);
    const active = company;
    const old = pending.current;
    const same =
      old?.company === target.company_id &&
      JSON.stringify({
        ...old.command,
        request_id: undefined,
        revision: undefined,
      }) === JSON.stringify(action);
    const command = same
      ? old.command
      : {
          ...action,
          revision: target.revision,
          request_id: crypto.randomUUID(),
        };
    pending.current = { company: target.company_id, command };
    try {
      const value = await companyAdministration(
        apiUrl,
        target.company_id,
        command,
      );
      pending.current = undefined;
      if (scope.current !== active) return false;
      if (
        action.action === "leave" ||
        action.action === "transfer" ||
        action.action === "restore"
      ) {
        window.location.reload();
        return;
      }
      setData(value);
      return true;
    } catch (e: unknown) {
      if (scope.current !== active) return false;
      if ((e as { status?: number }).status === 401) onExpired();
      else setError(true);
      return false;
    } finally {
      if (scope.current === active) setBusy(false);
    }
  }
  async function download() {
    if (!company || busy) return;
    setBusy(true);
    setError(false);
    try {
      const blob = await companyArchive(apiUrl, company);
      if (scope.current !== company) return;
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `company-${company}-archive.zip`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e: unknown) {
      if ((e as { status?: number }).status === 401) onExpired();
      else setError(true);
      return false;
    } finally {
      setBusy(false);
    }
  }
  return { data, trash, busy, error, execute, download };
}
