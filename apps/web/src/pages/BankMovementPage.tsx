import { BankForeignDialog } from "../components/organisms/BankForeignDialog";
import { BankSplitDialog } from "../components/organisms/BankSplitDialog";
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
  bankAccounts,
  bankGet,
  bankCandidates,
  bankSuggestions,
  SalesFailure,
  type BankAccount,
  type BankMovement,
  type BankCandidate,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useBankCommand } from "../hooks/useBankCommand";
import { useAppTranslation } from "../locales";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
export function BankMovementPage() {
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
    cmd = useBankCommand(companyId);
  const [movement, setMovement] = useState<BankMovement | null>(null),
    [accounts, setAccounts] = useState<BankAccount[]>([]),
    [candidates, setCandidates] = useState<BankCandidate[] | null>(null),
    [error, setError] = useState(false),
    [candidateError, setCandidateError] = useState(false),
    [attempt, setAttempt] = useState(0),
    [input, setInput] = useState(""),
    [query, setQuery] = useState(""),
    [selected, setSelected] = useState<BankCandidate | null>(null),
    [unlink, setUnlink] = useState(false),
    [reason, setReason] = useState("");
  const [split, setSplit] = useState(false);
  const [foreign, setForeign] = useState(false);
  const [suggestions, setSuggestions] = useState<BankCandidate[] | null>(null);
  const [suggestionError, setSuggestionError] = useState(false);
  useEffect(() => {
    const c = new AbortController();
    setSuggestions(null);
    setSuggestionError(false);
    bankSuggestions(apiUrl, companyId, movementId, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setSuggestions(v);
      })
      .catch(() => {
        if (!c.signal.aborted) setSuggestionError(true);
      });
    return () => c.abort();
  }, [companyId, movementId, attempt]);
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
      bankGet(apiUrl, companyId, movementId, c.signal),
      bankAccounts(apiUrl, companyId, c.signal),
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
    bankCandidates(apiUrl, companyId, movementId, query, c.signal)
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
        {t("bankFailed")}{" "}
        <Button onClick={() => setAttempt((x) => x + 1)}>
          {t("bankRetry")}
        </Button>
      </Alert>
    );
  if (!movement) return <p role="status">{t("bankLoading")}</p>;
  return (
    <div className="space-y-group">
      <Link className="text-sm underline" to={`/companies/${companyId}/bank`}>
        {t("bankBack")}
      </Link>
      <div className="flex flex-wrap justify-between items-center gap-control">
        <div className="flex items-center gap-control">
          <Heading level={1} size="page">
            {movement.number}
          </Heading>
          <Badge>
            {t(
              !active
                ? "bankUnmatched"
                : movement.remaining === "0.00"
                  ? "bankMatched"
                  : "bankPartial",
            )}
          </Badge>
        </div>
        <p className="text-xl font-semibold">
          {moneyText(movement.data.amount, language, movement.data.currency)}
        </p>
      </div>
      <section className="workspace-panel p-field">
        <dl className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-field">
          {[
            [
              t("bankAccount"),
              accounts.find((a) => a.id === movement.account_id)?.data.name,
            ],
            [
              t("bankDate"),
              formatCalendarDate(movement.data.booked_on, language),
            ],
            [t("bankVs"), movement.data.variable_symbol],
            [t("bankPartner"), movement.data.counterparty],
            [t("bankDescription"), movement.data.description],
            [t("bankExternalId"), movement.data.external_id],
          ].map(([label, value]) => (
            <div key={label} className="min-w-0">
              <dt className="text-sm text-muted">{label}</dt>
              <dd className="break-words">{value || "—"}</dd>
            </div>
          ))}
        </dl>
      </section>
      <section className="workspace-panel p-field space-y-field">
        <div className="flex flex-wrap justify-between gap-field">
          <p>
            {t("bankAllocated")}:{" "}
            <strong>
              {moneyText(movement.allocated, language, movement.data.currency)}
            </strong>
          </p>
          <p>
            {t("bankUnallocated")}:{" "}
            <strong>
              {moneyText(movement.remaining, language, movement.data.currency)}
            </strong>
          </p>
        </div>
        {canWrite &&
          movement.remaining !== "0.00" &&
          !accounts.find((a) => a.id === movement.account_id)?.archived && (
            <>
              <Button variant="secondary" onClick={() => setForeign(true)}>
                {t("bankForeignTitle")}
              </Button>
              <Button variant="secondary" onClick={() => setSplit(true)}>
                {t("bankSplit")}
              </Button>
            </>
          )}
      </section>
      {foreign && (
        <BankForeignDialog
          companyId={companyId}
          movement={movement}
          onClose={() => setForeign(false)}
          onSaved={(m) => {
            setMovement(m);
            setForeign(false);
            setAttempt((x) => x + 1);
          }}
        />
      )}
      {split && (
        <BankSplitDialog
          companyId={companyId}
          movement={movement}
          onClose={() => setSplit(false)}
          onSaved={(m) => {
            setMovement(m);
            setSplit(false);
            setAttempt((x) => x + 1);
          }}
        />
      )}
      {active ? (
        <section className="workspace-panel p-field space-y-field">
          <Heading level={2} size="section">
            {t("bankMatched")}
          </Heading>
          {movement.matches
            .filter((m) => !m.detached_at)
            .map((link) => (
              <div
                key={link.id}
                className="flex flex-wrap justify-between gap-control"
              >
                <div>
                  <Link
                    className="underline font-medium"
                    to={href(
                      link.target_kind,
                      link.document_id,
                      link.document_kind,
                    )}
                  >
                    {link.document_number}
                  </Link>
                  <p className="text-sm text-muted">
                    {t(
                      link.created_payment
                        ? "bankCreatedPayment"
                        : "bankExistingPayment",
                    )}
                  </p>
                </div>
                <div>
                  <strong>
                    {moneyText(link.amount, language, movement.data.currency)}
                  </strong>
                  {link.foreign && (
                    <p className="text-sm text-muted">
                      {t("bankForeignSummary", {
                        amount: moneyText(
                          link.foreign.document_amount,
                          language,
                          link.foreign.currency,
                        ),
                        book: moneyText(link.foreign.book_value_czk, language),
                        bank: moneyText(
                          link.foreign.bank_value_czk ?? link.amount,
                          language,
                        ),
                        difference: moneyText(
                          link.foreign.difference_czk,
                          language,
                        ),
                      })}
                    </p>
                  )}
                </div>
              </div>
            ))}
          {canWrite && (
            <Button
              variant="secondary"
              onClick={() => {
                cmd.setError("");
                setUnlink(true);
              }}
            >
              {t("bankUnmatch")}
            </Button>
          )}
        </section>
      ) : (
        <section className="workspace-panel register-sheet">
          <div className="p-field space-y-field">
            <Heading level={2} size="section">
              {t("bankMatchTitle")}
            </Heading>
            <p className="text-sm text-muted">
              {t(
                movement.data.currency === "CZK"
                  ? "bankCandidateHelp"
                  : "bankNativeMatchingHelp",
              )}
            </p>
            {accounts.find((a) => a.id === movement.account_id)?.archived && (
              <Alert>{t("bankArchivedHelp")}</Alert>
            )}
            <section
              className="space-y-control"
              aria-label={t("bankSuggestions")}
            >
              <Heading level={3}>{t("bankSuggestions")}</Heading>
              <p className="text-sm text-muted" role="status">
                {t(
                  suggestionError
                    ? "bankSuggestionsFailed"
                    : !suggestions
                      ? "bankLoading"
                      : suggestions.length === 0
                        ? "bankSuggestionsNone"
                        : suggestions.length === 1
                          ? "bankSuggestionsUnique"
                          : "bankSuggestionsAmbiguous",
                )}
              </p>
              {suggestions?.map((c) => (
                <div
                  key={`${c.document_id}:${c.payment_id ?? "new"}`}
                  className="flex flex-wrap items-center justify-between gap-control rounded-control border border-border p-control"
                >
                  <span>
                    {c.number} · {c.partner} ·{" "}
                    {t(
                      c.payment_id
                        ? "bankSuggestExisting"
                        : "bankCreatePayment",
                    )}
                  </span>
                  {canWrite && (
                    <Button
                      variant="secondary"
                      disabled={cmd.busy}
                      onClick={() => setSelected(c)}
                    >
                      {t("bankChooseSuggestion")}
                    </Button>
                  )}
                </div>
              ))}
            </section>
            <form
              className="flex flex-wrap gap-control"
              onSubmit={(e) => {
                e.preventDefault();
                setQuery(input);
              }}
            >
              <Input
                aria-label={t("bankCandidateSearch")}
                placeholder={t("bankCandidateSearch")}
                value={input}
                onChange={(e) => setInput(e.target.value)}
              />
              <Button variant="secondary" type="submit">
                {t("bankSearchAction")}
              </Button>
            </form>
          </div>
          {candidateError ? (
            <Alert tone="danger">
              {t("bankFailed")}
              <Button onClick={() => setAttempt((x) => x + 1)}>
                {t("bankRetry")}
              </Button>
            </Alert>
          ) : !candidates ? (
            <p role="status" className="p-field">
              {t("bankLoading")}
            </p>
          ) : (
            <>
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("bankDocument")}</TableHead>
                      <TableHead>{t("bankPartner")}</TableHead>
                      <TableHead>{t("bankPayment")}</TableHead>
                      <TableHead className="text-right">
                        {t("bankRemaining")}
                      </TableHead>
                      {canWrite && <TableHead>{t("bankAction")}</TableHead>}
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
                              ? "bankUseExisting"
                              : "bankCreatePayment",
                          )}
                          {c.payment_id && (
                            <p className="text-sm text-muted">
                              {t("bankPaymentId", { id: c.payment_id })}
                            </p>
                          )}
                        </TableCell>
                        <TableCell className="text-right whitespace-nowrap">
                          {moneyText(
                            c.outstanding,
                            language,
                            movement.data.currency,
                          )}
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
                              {t("bankMatch")}
                            </Button>
                          </TableCell>
                        )}
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
              {!candidates.length && (
                <p className="p-field text-muted">{t("bankNoCandidates")}</p>
              )}
            </>
          )}
        </section>
      )}
      {movement.matches.length > 0 && (
        <section className="workspace-panel p-field space-y-field">
          <Heading level={2} size="section">
            {t("bankHistory")}
          </Heading>
          <ul className="space-y-field">
            {movement.matches.map((m) => (
              <li key={m.id} className="space-y-inline">
                {m.foreign && (
                  <p className="text-sm text-muted">
                    {t("bankForeignSummary", {
                      amount: moneyText(
                        m.foreign.document_amount,
                        language,
                        m.foreign.currency,
                      ),
                      book: moneyText(m.foreign.book_value_czk, language),
                      bank: moneyText(
                        m.foreign.bank_value_czk ?? m.amount,
                        language,
                      ),
                      difference: moneyText(m.foreign.difference_czk, language),
                    })}
                  </p>
                )}
                <Link
                  className="underline"
                  to={href(m.target_kind, m.document_id, m.document_kind)}
                >
                  {m.document_number}
                </Link>{" "}
                · {moneyText(m.amount, language, movement.data.currency)}
                <p className="text-sm text-muted">
                  {new Intl.DateTimeFormat(language, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  }).format(new Date(m.created_at))}{" "}
                  ·{" "}
                  {t(
                    m.created_payment
                      ? "bankCreatedPayment"
                      : "bankExistingPayment",
                  )}
                </p>
                {m.detached_at && (
                  <p className="text-sm">
                    {t("bankDetached")} ·{" "}
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
        open={!!selected || unlink}
        onClose={() => {
          if (!cmd.busy) {
            setSelected(null);
            setUnlink(false);
            setReason("");
          }
        }}
        title={t(unlink ? "bankUnmatch" : "bankMatch")}
        description={t(
          unlink
            ? "bankUnmatchAllHelp"
            : selected?.payment_id
              ? "bankMatchExistingHelp"
              : "bankMatchNewHelp",
        )}
      >
        <form
          className="space-y-field"
          onSubmit={async (e) => {
            e.preventDefault();
            const r = await cmd.execute(
              unlink
                ? {
                    request_id: "",
                    action: "unmatch",
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
              setReason("");
              setAttempt((x) => x + 1);
            }
          }}
        >
          {cmd.error && <Alert tone="danger">{t(cmd.error)}</Alert>}
          {selected && (
            <p className="font-medium">
              {selected.number} ·{" "}
              {moneyText(
                movement.data.amount,
                language,
                movement.data.currency,
              )}
            </p>
          )}
          {unlink && (
            <label className="block space-y-inline">
              <span>{t("bankReason")}</span>
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
                setReason("");
              }}
            >
              {t("bankClose")}
            </Button>
            <Button type="submit" disabled={cmd.busy || (!selected && !unlink)}>
              {t(
                cmd.busy ? "bankSaving" : unlink ? "bankUnmatch" : "bankMatch",
              )}
            </Button>
          </div>
        </form>
      </Dialog>
    </div>
  );
}
