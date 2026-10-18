import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Alert,
  Badge,
  Button,
  Heading,
  Input,
  Select,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  bankAccounts,
  bankMovements,
  SalesFailure,
  type BankAccount,
  type BankPage as Page,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation } from "../locales";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
import { BankForms, type BankForm } from "../components/organisms/BankForms";
import { BankBatchDialog } from "../components/organisms/BankBatchDialog";
import { BankBalancePanel } from "../components/organisms/BankBalancePanel";
import { BankConnectionDialog } from "../components/organisms/BankConnectionDialog";
export function BankPage() {
  const { companyId = "" } = useParams();
  return <BankRegister key={companyId} companyId={companyId} />;
}
function BankRegister({ companyId }: { companyId: string }) {
  const { company, onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "",
    account = params.get("account") ?? "",
    direction = params.get("direction") ?? "",
    status = params.get("status") ?? "",
    page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1));
  const [input, setInput] = useState(query),
    [accounts, setAccounts] = useState<BankAccount[]>([]),
    [result, setResult] = useState<Page | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0),
    [form, setForm] = useState<BankForm | null>(null),
    [showAccounts, setShowAccounts] = useState(false),
    [saved, setSaved] = useState(false);
  const [connectionOpen, setConnectionOpen] = useState(false);
  const [batchIds, setBatchIds] = useState<string[] | null>(null);
  const canWrite = company?.id === companyId && company.role !== "viewer";
  const filter = (change: Record<string, string>) =>
    setParams({ q: query, account, direction, status, ...change });
  useEffect(() => setInput(query), [query]);
  useEffect(() => {
    const c = new AbortController();
    setError(false);
    setResult(null);
    Promise.all([
      bankAccounts(apiUrl, companyId, c.signal),
      bankMovements(
        apiUrl,
        companyId,
        { query, account_id: account, direction, status, page: String(page) },
        c.signal,
      ),
    ])
      .then(([a, r]) => {
        if (!c.signal.aborted) {
          setAccounts(a);
          setResult(r);
        }
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, query, account, direction, status, page, attempt]);
  return (
    <div className="space-y-group">
      <div className="flex flex-wrap items-center justify-between gap-field">
        <Heading level={1} size="page">
          {t("navBank")}
        </Heading>
        <div className="flex flex-wrap gap-control">
          <Button
            variant="secondary"
            onClick={() => setShowAccounts(!showAccounts)}
            aria-expanded={showAccounts}
          >
            {t("bankAccounts")}
          </Button>
          <Button
            variant="secondary"
            disabled={!accounts.length}
            onClick={() => setConnectionOpen(true)}
          >
            {t("connectionTitle")}
          </Button>
          {canWrite && (
            <>
              <Button
                variant="secondary"
                disabled={
                  !result?.items.some(
                    (m) => !m.matches.some((x) => !x.detached_at),
                  )
                }
                onClick={() => {
                  setSaved(false);
                  setBatchIds(
                    result!.items
                      .filter((m) => !m.matches.some((x) => !x.detached_at))
                      .map((m) => m.id),
                  );
                }}
              >
                {t("bankBatchOpen")}
              </Button>
              <Button
                variant="secondary"
                disabled={!accounts.some((a) => !a.archived)}
                onClick={() => {
                  setSaved(false);
                  setForm({ kind: "manual" });
                }}
              >
                {t("bankManual")}
              </Button>
              <Button
                disabled={!accounts.some((a) => !a.archived)}
                onClick={() => {
                  setSaved(false);
                  setForm({ kind: "import" });
                }}
              >
                {t("bankImport")}
              </Button>
            </>
          )}
        </div>
      </div>
      {connectionOpen && (
        <BankConnectionDialog
          companyId={companyId}
          accounts={accounts}
          onClose={() => setConnectionOpen(false)}
          onSynced={() => setAttempt((x) => x + 1)}
        />
      )}
      {saved && <Alert tone="success">{t("bankSaved")}</Alert>}
      {error ? (
        <Alert tone="danger">
          {t("bankFailed")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((x) => x + 1)}>
            {t("bankRetry")}
          </Button>
        </Alert>
      ) : !result ? (
        <p role="status">{t("bankLoading")}</p>
      ) : (
        <>
          <BankBalancePanel
            companyId={companyId}
            accounts={accounts}
            version={attempt}
            onSaved={() => setAttempt((x) => x + 1)}
          />
          {(showAccounts || !accounts.length) && (
            <section className="workspace-panel p-field space-y-field">
              <div className="flex flex-wrap justify-between gap-control">
                <Heading level={2} size="section">
                  {t("bankAccounts")}
                </Heading>
                {canWrite && (
                  <Button
                    variant="secondary"
                    onClick={() => {
                      setSaved(false);
                      setForm({ kind: "account" });
                    }}
                  >
                    {t("bankAddAccount")}
                  </Button>
                )}
              </div>
              {!accounts.length && (
                <p className="text-muted">{t("bankNoAccounts")}</p>
              )}
              <ul className="divide-y divide-border">
                {accounts.map((a) => (
                  <li
                    key={a.id}
                    className="py-field flex flex-wrap justify-between items-center gap-control"
                  >
                    <div className="min-w-0">
                      <span className="font-medium">{a.data.name}</span>
                      {a.archived && <Badge>{t("bankArchived")}</Badge>}
                      <p className="text-sm text-muted break-all">
                        {a.data.iban} · {a.data.currency}
                      </p>
                    </div>
                    {canWrite && (
                      <Button
                        variant="secondary"
                        onClick={() => setForm({ kind: "account", account: a })}
                      >
                        {t("bankEdit")}
                      </Button>
                    )}
                  </li>
                ))}
              </ul>
            </section>
          )}
          <section className="workspace-panel register-sheet">
            <form
              className="p-field flex flex-wrap items-end gap-field"
              onSubmit={(e) => {
                e.preventDefault();
                filter({ q: input });
              }}
            >
              <label className="block space-y-inline">
                <span className="text-sm">{t("bankSearch")}</span>
                <Input
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                />
              </label>
              <Button type="submit" variant="secondary">
                {t("bankSearchAction")}
              </Button>
              <Select
                label={t("bankAccount")}
                value={account}
                onChange={(v) => filter({ account: v })}
                options={[
                  { value: "", label: t("bankAll") },
                  ...accounts.map((a) => ({ value: a.id, label: a.data.name })),
                ]}
              />
              <Select
                label={t("bankDirection")}
                value={direction}
                onChange={(v) => filter({ direction: v })}
                options={[
                  { value: "", label: t("bankAll") },
                  { value: "income", label: t("bankIncome") },
                  { value: "expense", label: t("bankExpense") },
                ]}
              />
              <Select
                label={t("bankStatus")}
                value={status}
                onChange={(v) => filter({ status: v })}
                options={[
                  { value: "", label: t("bankAll") },
                  { value: "unmatched", label: t("bankUnmatched") },
                  { value: "matched", label: t("bankMatched") },
                  { value: "partial", label: t("bankPartial") },
                ]}
              />
            </form>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("bankDocument")}</TableHead>
                    <TableHead>{t("bankDate")}</TableHead>
                    <TableHead>{t("bankPartner")}</TableHead>
                    <TableHead>{t("bankVs")}</TableHead>
                    <TableHead>{t("bankAccount")}</TableHead>
                    <TableHead>{t("bankStatus")}</TableHead>
                    <TableHead className="text-right">
                      {t("bankAmount")}
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <tbody>
                  {result.items.map((m) => (
                    <TableRow key={m.id}>
                      <TableCell>
                        <Link
                          className="font-medium underline-offset-4 hover:underline"
                          to={m.id}
                        >
                          {m.number}
                        </Link>
                        <p className="text-sm text-muted max-w-64 truncate">
                          {m.data.description}
                        </p>
                      </TableCell>
                      <TableCell className="whitespace-nowrap">
                        {formatCalendarDate(m.data.booked_on, language)}
                      </TableCell>
                      <TableCell>{m.data.counterparty || "—"}</TableCell>
                      <TableCell>{m.data.variable_symbol || "—"}</TableCell>
                      <TableCell>
                        {accounts.find((a) => a.id === m.account_id)?.data.name}
                      </TableCell>
                      <TableCell>
                        <Badge>
                          {t(
                            m.allocated === "0.00"
                              ? "bankUnmatched"
                              : m.remaining === "0.00"
                                ? "bankMatched"
                                : "bankPartial",
                          )}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right whitespace-nowrap font-medium">
                        {moneyText(m.data.amount, language, m.data.currency)}
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!result.items.length && (
              <p className="p-field text-muted">{t("bankEmpty")}</p>
            )}
            <div className="p-field flex flex-wrap justify-end items-center gap-control">
              <span>{t("quotePage", { page })}</span>
              <Button
                variant="secondary"
                disabled={page === 1}
                onClick={() => filter({ page: String(page - 1) })}
              >
                {t("quotePrevious")}
              </Button>
              <Button
                variant="secondary"
                disabled={page * 25 >= result.total}
                onClick={() => filter({ page: String(page + 1) })}
              >
                {t("quoteNext")}
              </Button>
            </div>
          </section>
        </>
      )}
      {batchIds && (
        <BankBatchDialog
          companyId={companyId}
          movementIds={batchIds}
          accounts={accounts}
          onClose={() => setBatchIds(null)}
          onSaved={() => {
            setBatchIds(null);
            setSaved(true);
            setAttempt((v) => v + 1);
          }}
        />
      )}
      {form && (
        <BankForms
          form={form}
          companyId={companyId}
          accounts={accounts}
          onClose={() => setForm(null)}
          onSaved={() => {
            setForm(null);
            setSaved(true);
            setAttempt((x) => x + 1);
          }}
        />
      )}
    </div>
  );
}
