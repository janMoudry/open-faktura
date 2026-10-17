import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import {
  Alert,
  Badge,
  Button,
  Dialog,
  Heading,
  Input,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  cashAccounts,
  cashGet,
  cashCandidates,
  SalesFailure,
  type CashAccount,
  type CashMovement,
  type CashCandidate,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useCashCommand } from "../hooks/useCashCommand";
import { useAppTranslation } from "../locales";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
import { CashDocumentOutput } from "../components/organisms/CashDocumentOutput";
export function CashMovementPage() {
  const { companyId = "", movementId = "" } = useParams();
  return (
    <MovementDetail
      key={`${companyId}:${movementId}`}
      companyId={companyId}
      movementId={movementId}
    />
  );
}
function MovementDetail({
  companyId,
  movementId,
}: {
  companyId: string;
  movementId: string;
}) {
  const { company, onExpired, setPageDirty } = useWorkspace(),
    { t, language } = useAppTranslation(),
    cmd = useCashCommand(companyId);
  const [movement, setMovement] = useState<CashMovement | null>(null),
    [accounts, setAccounts] = useState<CashAccount[]>([]),
    [candidates, setCandidates] = useState<CashCandidate[] | null>(null),
    [error, setError] = useState(false),
    [candidateError, setCandidateError] = useState(false),
    [attempt, setAttempt] = useState(0),
    [input, setInput] = useState(""),
    [query, setQuery] = useState(""),
    [selected, setSelected] = useState<CashCandidate | null>(null),
    [unlink, setUnlink] = useState(false),
    [cancel, setCancel] = useState(false),
    [reason, setReason] = useState("");
  const canWrite = company?.id === companyId && company.role !== "viewer",
    active = movement?.matches.find((x) => !x.detached_at);
  useEffect(() => {
    setPageDirty(!!reason);
    return () => setPageDirty(false);
  }, [reason, setPageDirty]);
  useEffect(() => {
    const c = new AbortController();
    setError(false);
    Promise.all([
      cashGet(apiUrl, companyId, movementId, c.signal),
      cashAccounts(apiUrl, companyId, c.signal),
    ])
      .then(([m, a]) => {
        if (!c.signal.aborted) {
          setMovement(m);
          setAccounts(a);
        }
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, movementId, attempt]);
  useEffect(() => {
    if (!movement || active) return;
    const c = new AbortController();
    setCandidates(null);
    setCandidateError(false);
    cashCandidates(apiUrl, companyId, movementId, query, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setCandidates(v);
      })
      .catch(() => {
        if (!c.signal.aborted) setCandidateError(true);
      });
    return () => c.abort();
  }, [companyId, movementId, movement?.revision, query, attempt]);
  const href = (kind: string, id: string, docKind: string) =>
    `/companies/${companyId}/${kind === "advance" ? "advances" : kind === "credit" ? "credit-notes" : kind === "invoice" ? "invoices" : docKind === "receipt" ? "receipts" : "received-invoices"}/${id}`;
  if (error)
    return (
      <Alert tone="danger">
        {t("cashFailed")}{" "}
        <Button onClick={() => setAttempt((x) => x + 1)}>
          {t("cashRetry")}
        </Button>
      </Alert>
    );
  if (!movement) return <p role="status">{t("cashLoading")}</p>;
  return (
    <div className="space-y-group">
      <Link className="text-sm underline" to={`/companies/${companyId}/cash`}>
        {t("cashBack")}
      </Link>
      <div className="flex flex-wrap justify-between items-center gap-control">
        <div className="flex items-center gap-control">
          <Heading level={1} size="page">
            {movement.number}
          </Heading>
          <Badge>
            {t(
              movement.cancelled
                ? "cashCancelled"
                : movement.tax
                  ? "cashTaxBadge"
                  : active
                    ? "cashMatched"
                    : "cashUnmatched",
            )}
          </Badge>
        </div>
        <p className="text-xl font-semibold">
          {moneyText(movement.data.amount, language)}
        </p>
      </div>
      <CashDocumentOutput
        key={movement.revision}
        companyId={companyId}
        movement={movement}
      />
      {movement.retail_posting_id && (
        <Alert>
          {t("retailPosted")} —{" "}
          <Link to={`/companies/${companyId}/retail`}>
            {t("retailPostingTitle")}
          </Link>
        </Alert>
      )}
      {movement.automatic && <Alert>{t("cashAutomaticSource")}</Alert>}
      <section className="workspace-panel p-field">
        <dl className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-field">
          {[
            [
              t("cashAccount"),
              accounts.find((a) => a.id === movement.account_id)?.data.name,
            ],
            [
              t("cashDate"),
              formatCalendarDate(movement.data.booked_on, language),
            ],
            [t("cashVs"), movement.data.variable_symbol],
            [t("cashPartner"), movement.data.counterparty],
            [t("cashDescription"), movement.data.description],
            [t("cashPerson"), movement.data.person],
          ].map(([label, value]) => (
            <div key={label} className="min-w-0">
              <dt className="text-sm text-muted">{label}</dt>
              <dd className="break-words">{value || "—"}</dd>
            </div>
          ))}
        </dl>
      </section>
      {movement.tax && (
        <section className="workspace-panel p-field space-y-field">
          <Heading level={2} size="section">
            {t("cashTaxBreakdown")}
          </Heading>
          <Alert>{t("cashTaxHelp")}</Alert>
          <dl className="grid grid-cols-2 gap-field md:grid-cols-3">
            {[
              [
                t("cashTaxDate"),
                formatCalendarDate(movement.tax.tax.tax_on, language),
              ],
              [t("cashTaxRate"), movement.tax.tax.vat_percent + " %"],
              [t("cashTaxNet"), moneyText(movement.tax.amounts.net, language)],
              [t("cashTaxVat"), moneyText(movement.tax.amounts.vat, language)],
              [
                t("cashTaxTotal"),
                moneyText(movement.tax.amounts.gross, language),
              ],
            ].map(([label, value]) => (
              <div key={label}>
                <dt className="text-sm text-muted">{label}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
        </section>
      )}
      {movement.cancelled ? (
        <Alert>
          {t("cashCancelled")} · {movement.reason}
        </Alert>
      ) : active ? (
        <section className="workspace-panel p-field space-y-field">
          <Heading level={2} size="section">
            {t("cashMatched")}
          </Heading>
          <Link
            className="underline font-medium"
            to={href(
              active.target_kind,
              active.document_id,
              active.document_kind,
            )}
          >
            {active.document_number}
          </Link>
          <p className="text-muted">
            {t(
              active.created_payment
                ? "cashCreatedPayment"
                : "cashExistingPayment",
            )}
          </p>
          {canWrite && !movement.automatic && !movement.retail_posting_id && (
            <Button
              variant="secondary"
              onClick={() => {
                cmd.setError("");
                setUnlink(true);
              }}
            >
              {t("cashUnmatch")}
            </Button>
          )}
        </section>
      ) : movement.tax ? null : (
        <section className="workspace-panel register-sheet">
          <div className="p-field space-y-field">
            <Heading level={2} size="section">
              {t("cashMatchTitle")}
            </Heading>
            <p className="text-sm text-muted">{t("cashCandidateHelp")}</p>
            {accounts.find((a) => a.id === movement.account_id)?.archived && (
              <Alert>{t("cashArchivedHelp")}</Alert>
            )}
            <form
              className="flex flex-wrap gap-control"
              onSubmit={(e) => {
                e.preventDefault();
                setQuery(input);
              }}
            >
              <Input
                aria-label={t("cashCandidateSearch")}
                placeholder={t("cashCandidateSearch")}
                value={input}
                onChange={(e) => setInput(e.target.value)}
              />
              <Button variant="secondary" type="submit">
                {t("cashSearchAction")}
              </Button>
            </form>
          </div>
          {candidateError ? (
            <Alert tone="danger">
              {t("cashFailed")}
              <Button onClick={() => setAttempt((x) => x + 1)}>
                {t("cashRetry")}
              </Button>
            </Alert>
          ) : !candidates ? (
            <p role="status" className="p-field">
              {t("cashLoading")}
            </p>
          ) : (
            <>
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("cashDocument")}</TableHead>
                      <TableHead>{t("cashPartner")}</TableHead>
                      <TableHead>{t("cashPayment")}</TableHead>
                      <TableHead className="text-right">
                        {t("cashRemaining")}
                      </TableHead>
                      {canWrite && <TableHead>{t("cashAction")}</TableHead>}
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {candidates.map((c) => (
                      <TableRow
                        key={`${c.target_kind}:${c.document_id}:${c.payment_id ?? "new"}`}
                      >
                        <TableCell>
                          <Link
                            className="underline"
                            to={href(
                              c.target_kind,
                              c.document_id,
                              c.document_kind,
                            )}
                          >
                            {c.number}
                          </Link>
                        </TableCell>
                        <TableCell>{c.partner}</TableCell>
                        <TableCell>
                          {t(
                            c.payment_id
                              ? "cashUseExisting"
                              : "cashCreatePayment",
                          )}
                          {c.payment_id && (
                            <p className="text-sm text-muted">
                              {t("cashPaymentId", { id: c.payment_id })}
                            </p>
                          )}
                        </TableCell>
                        <TableCell className="text-right whitespace-nowrap">
                          {moneyText(c.outstanding, language)}
                        </TableCell>
                        {canWrite && (
                          <TableCell>
                            <Button
                              variant="secondary"
                              onClick={() => {
                                cmd.setError("");
                                setSelected(c);
                              }}
                            >
                              {t("cashMatch")}
                            </Button>
                          </TableCell>
                        )}
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
              {!candidates.length && (
                <p className="p-field text-muted">{t("cashNoCandidates")}</p>
              )}
            </>
          )}
        </section>
      )}
      {canWrite &&
        !active &&
        !movement.cancelled &&
        !movement.automatic &&
        !movement.retail_posting_id && (
          <Button
            variant="secondary"
            onClick={() => {
              cmd.setError("");
              setCancel(true);
            }}
          >
            {t("cashCancel")}
          </Button>
        )}
      {movement.matches.length > 0 && (
        <section className="workspace-panel p-field space-y-field">
          <Heading level={2} size="section">
            {t("cashHistory")}
          </Heading>
          <ul className="space-y-field">
            {movement.matches.map((m) => (
              <li key={m.id} className="space-y-inline">
                <Link
                  className="underline"
                  to={href(m.target_kind, m.document_id, m.document_kind)}
                >
                  {m.document_number}
                </Link>
                <p className="text-sm text-muted">
                  {new Intl.DateTimeFormat(language, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  }).format(new Date(m.created_at))}{" "}
                  ·{" "}
                  {t(
                    m.created_payment
                      ? "cashCreatedPayment"
                      : "cashExistingPayment",
                  )}
                </p>
                {m.detached_at && (
                  <p className="text-sm">
                    {t("cashDetached")} ·{" "}
                    {new Intl.DateTimeFormat(language, {
                      dateStyle: "medium",
                      timeStyle: "short",
                    }).format(new Date(m.detached_at))}{" "}
                    · {m.reason}
                  </p>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}
      <Dialog
        open={!!selected || unlink || cancel}
        onClose={() => {
          if (!cmd.busy) {
            setSelected(null);
            setUnlink(false);
            setCancel(false);
            setReason("");
          }
        }}
        title={t(cancel ? "cashCancel" : unlink ? "cashUnmatch" : "cashMatch")}
        description={t(
          cancel
            ? "cashCancelHelp"
            : unlink
              ? active?.created_payment
                ? "cashUnmatchCreated"
                : "cashUnmatchExisting"
              : selected?.payment_id
                ? "cashMatchExistingHelp"
                : "cashMatchNewHelp",
        )}
      >
        <form
          className="space-y-field"
          onSubmit={async (e) => {
            e.preventDefault();
            const r = await cmd.execute(
              cancel || unlink
                ? {
                    request_id: "",
                    action: cancel ? "cancel" : "unmatch",
                    movement_id: movement.id,
                    revision: movement.revision,
                    reason,
                  }
                : {
                    request_id: "",
                    action: "match",
                    movement_id: movement.id,
                    revision: movement.revision,
                    target_kind: selected!.target_kind,
                    document_id: selected!.document_id,
                    target_revision: selected!.revision,
                    payment_id: selected!.payment_id,
                  },
            );
            if (r?.kind === "movement") {
              setMovement(r.value);
              setSelected(null);
              setUnlink(false);
              setCancel(false);
              setReason("");
              setAttempt((x) => x + 1);
            }
          }}
        >
          {cmd.error && <Alert tone="danger">{t(cmd.error)}</Alert>}
          {selected && (
            <p className="font-medium">
              {selected.number} · {moneyText(movement.data.amount, language)}
            </p>
          )}
          {(unlink || cancel) && (
            <label className="block space-y-inline">
              <span>{t("cashReason")}</span>
              <Input
                required
                value={reason}
                maxLength={500}
                onChange={(e) => setReason(e.target.value)}
                disabled={cmd.busy}
              />
            </label>
          )}
          <div className="flex justify-end gap-control">
            <Button
              type="button"
              variant="secondary"
              disabled={cmd.busy}
              onClick={() => {
                setSelected(null);
                setUnlink(false);
                setCancel(false);
                setReason("");
              }}
            >
              {t("cashClose")}
            </Button>
            <Button
              type="submit"
              disabled={cmd.busy || (!selected && !unlink && !cancel)}
            >
              {t(
                cmd.busy
                  ? "cashSaving"
                  : cancel
                    ? "cashCancel"
                    : unlink
                      ? "cashUnmatch"
                      : "cashMatch",
              )}
            </Button>
          </div>
        </form>
      </Dialog>
    </div>
  );
}
