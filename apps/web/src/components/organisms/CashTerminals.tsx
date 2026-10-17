import { useEffect, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Field,
  Heading,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  cashTerminals,
  SalesFailure,
  type CashTerminal,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useCashCommand } from "../../hooks/useCashCommand";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
export function CashTerminals({
  companyId,
  canWrite,
}: {
  companyId: string;
  canWrite: boolean;
}) {
  const { t } = useAppTranslation(),
    { onExpired } = useWorkspace(),
    cmd = useCashCommand(companyId);
  const [items, setItems] = useState<CashTerminal[] | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0),
    [editing, setEditing] = useState<CashTerminal | null>(null),
    [code, setCode] = useState(""),
    [name, setName] = useState(""),
    [saved, setSaved] = useState(false);
  useEffect(() => {
    const c = new AbortController();
    setError(false);
    cashTerminals(apiUrl, companyId, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setItems(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, attempt]);
  const reset = () => {
    setEditing(null);
    setCode("");
    setName("");
    cmd.setError("");
  };
  return (
    <section
      aria-label={t("terminalTitle")}
      className="workspace-panel min-w-0 p-field space-y-field"
    >
      <Heading level={2} size="section">
        {t("terminalTitle")}
      </Heading>
      <p className="text-sm text-muted">{t("terminalHelp")}</p>
      {error ? (
        <Alert tone="danger">
          {t("cashFailed")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((v) => v + 1)}>
            {t("cashRetry")}
          </Button>
        </Alert>
      ) : !items ? (
        <p role="status">{t("cashLoading")}</p>
      ) : (
        <>
          {!!items.length && (
            <div className="overflow-x-auto">
              <Table className="whitespace-nowrap">
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("terminalCode")}</TableHead>
                    <TableHead>{t("terminalName")}</TableHead>
                    <TableHead>{t("terminalStatus")}</TableHead>
                    <TableHead>{t("terminalActions")}</TableHead>
                  </TableRow>
                </TableHeader>
                <tbody>
                  {items.map((v) => (
                    <TableRow key={v.id}>
                      <TableCell>{v.code}</TableCell>
                      <TableCell>{v.name}</TableCell>
                      <TableCell>
                        <Badge>
                          {t(
                            v.archived ? "terminalArchived" : "terminalActive",
                          )}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        {canWrite && (
                          <div className="flex gap-control whitespace-nowrap">
                            <Button
                              variant="ghost"
                              disabled={cmd.busy}
                              onClick={() => {
                                setEditing(v);
                                setCode(v.code);
                                setName(v.name);
                                setSaved(false);
                                cmd.setError("");
                              }}
                            >
                              {t("cashEdit")}
                            </Button>
                            <Button
                              variant="ghost"
                              disabled={cmd.busy}
                              onClick={async () => {
                                setSaved(false);
                                const r = await cmd.execute({
                                  request_id: "",
                                  action: "terminal",
                                  ...v,
                                  archived: !v.archived,
                                });
                                if (r?.kind === "terminal") {
                                  reset();
                                  setAttempt((n) => n + 1);
                                  setSaved(true);
                                }
                              }}
                            >
                              {t(
                                v.archived
                                  ? "terminalRestore"
                                  : "terminalArchive",
                              )}
                            </Button>
                          </div>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
          )}
          {canWrite && (
            <form
              className="grid grid-cols-1 items-end gap-field sm:grid-cols-2 lg:grid-cols-4"
              onSubmit={async (e) => {
                e.preventDefault();
                if (cmd.busy) return;
                setSaved(false);
                const r = await cmd.execute({
                  request_id: "",
                  action: "terminal",
                  id: editing?.id ?? null,
                  revision: editing?.revision ?? null,
                  code,
                  name,
                  archived: editing?.archived ?? false,
                });
                if (r?.kind === "terminal") {
                  reset();
                  setAttempt((v) => v + 1);
                  setSaved(true);
                }
              }}
            >
              <Field
                label={t("terminalCode")}
                required
                maxLength={32}
                pattern="[A-Za-z0-9_-]+"
                value={code}
                disabled={cmd.busy}
                onChange={(e) => setCode(e.target.value)}
              />
              <Field
                label={t("terminalName")}
                required
                maxLength={100}
                value={name}
                disabled={cmd.busy}
                onChange={(e) => setName(e.target.value)}
              />
              <Button type="submit" disabled={cmd.busy}>
                {t(
                  cmd.busy
                    ? "cashSaving"
                    : editing
                      ? "cashSave"
                      : "terminalAdd",
                )}
              </Button>
              {editing && (
                <Button variant="ghost" disabled={cmd.busy} onClick={reset}>
                  {t("terminalCancel")}
                </Button>
              )}
            </form>
          )}
        </>
      )}
      {cmd.error && (
        <Alert tone="danger">
          {t(cmd.error)}{" "}
          <Button
            variant="secondary"
            onClick={() => {
              reset();
              setAttempt((v) => v + 1);
            }}
          >
            {t("cashRetry")}
          </Button>
        </Alert>
      )}
      {saved && <Alert tone="success">{t("cashSaved")}</Alert>}
    </section>
  );
}
