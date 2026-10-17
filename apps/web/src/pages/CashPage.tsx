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
  cashAccounts,
  cashMovements,
  SalesFailure,
  type CashAccount,
  type CashPage as Page,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation } from "../locales";
import { apiUrl } from "../services/status";
import { moneyText } from "../utils/invoice";
import { CashForms, type CashForm } from "../components/organisms/CashForms";
import { CashBalancePanel } from "../components/organisms/CashBalancePanel";
import { CashTerminals } from "../components/organisms/CashTerminals";
import { CashAutomation } from "../components/organisms/CashAutomation";
export function CashPage() {
  const { companyId = "" } = useParams();
  return <CashRegister key={companyId} companyId={companyId} />;
}
function CashRegister({ companyId }: { companyId: string }) {
  const { company, onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "",
    account = params.get("account") ?? "",
    direction = params.get("direction") ?? "",
    status = params.get("status") ?? "",
    page = Math.max(1, Math.min(10000, Number(params.get("page")) || 1));
  const [input, setInput] = useState(query),
    [accounts, setAccounts] = useState<CashAccount[]>([]),
    [result, setResult] = useState<Page | null>(null),
    [error, setError] = useState(false),
    [attempt, setAttempt] = useState(0),
    [form, setForm] = useState<CashForm | null>(null),
    [showAccounts, setShowAccounts] = useState(false),
    [saved, setSaved] = useState(false);

  const canWrite = company?.id === companyId && company.role !== "viewer";
  const filter = (change: Record<string, string>) =>
    setParams({ q: query, account, direction, status, ...change });
  useEffect(() => setInput(query), [query]);
  useEffect(() => {
    const c = new AbortController();
    setError(false);
    setResult(null);
    Promise.all([
      cashAccounts(apiUrl, companyId, c.signal),
      cashMovements(
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
          {t("navCash")}
        </Heading>
        <div className="flex flex-wrap gap-control">
          <Button
            variant="secondary"
            onClick={() => setShowAccounts(!showAccounts)}
            aria-expanded={showAccounts}
          >
            {t("cashAccounts")}
          </Button>
          {canWrite && (
            <>
              <Button
                variant="secondary"
                disabled={!accounts.some((a) => !a.archived)}
                onClick={() => {
                  setSaved(false);
                  setForm({ kind: "manual" });
                }}
              >
                {t("cashManual")}
              </Button>
            </>
          )}
        </div>
      </div>
      {saved && <Alert tone="success">{t("cashSaved")}</Alert>}
      {error ? (
        <Alert tone="danger">
          {t("cashFailed")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((x) => x + 1)}>
            {t("cashRetry")}
          </Button>
        </Alert>
      ) : !result ? (
        <p role="status">{t("cashLoading")}</p>
      ) : (
        <>
          {(showAccounts || !accounts.length) && (
            <section className="workspace-panel p-field space-y-field">
              <div className="flex flex-wrap justify-between gap-control">
                <Heading level={2} size="section">
                  {t("cashAccounts")}
                </Heading>
                {canWrite && (
                  <Button
                    variant="secondary"
                    onClick={() => {
                      setSaved(false);
                      setForm({ kind: "account" });
                    }}
                  >
                    {t("cashAddAccount")}
                  </Button>
                )}
              </div>
              {!accounts.length && (
                <p className="text-muted">{t("cashNoAccounts")}</p>
              )}
              <ul className="divide-y divide-border">
                {accounts.map((a) => (
                  <li
                    key={a.id}
                    className="py-field flex flex-wrap justify-between items-center gap-control"
                  >
                    <div className="min-w-0">
                      <span className="font-medium">{a.data.name}</span>
                      {a.archived && <Badge>{t("cashArchived")}</Badge>}
                      <p className="text-sm text-muted break-all">
                        {a.data.currency}
                      </p>
                    </div>
                    {canWrite && (
                      <Button
                        variant="secondary"
                        onClick={() => setForm({ kind: "account", account: a })}
                      >
                        {t("cashEdit")}
                      </Button>
                    )}
                  </li>
                ))}
              </ul>
            </section>
          )}
          {showAccounts && (
            <CashTerminals companyId={companyId} canWrite={canWrite} />
          )}
          <CashAutomation
            companyId={companyId}
            accounts={accounts}
            canWrite={canWrite}
            version={attempt}
          />
          <CashBalancePanel
            companyId={companyId}
            accounts={accounts}
            version={attempt}
          />
          <section className="workspace-panel register-sheet">
            <form
              className="p-field flex flex-wrap items-end gap-field"
              onSubmit={(e) => {
                e.preventDefault();
                filter({ q: input });
              }}
            >
              <label className="block space-y-inline">
                <span className="text-sm">{t("cashSearch")}</span>
                <Input
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                />
              </label>
              <Button type="submit" variant="secondary">
                {t("cashSearchAction")}
              </Button>
              <Select
                label={t("cashAccount")}
                value={account}
                onChange={(v) => filter({ account: v })}
                options={[
                  { value: "", label: t("cashAll") },
                  ...accounts.map((a) => ({ value: a.id, label: a.data.name })),
                ]}
              />
              <Select
                label={t("cashDirection")}
                value={direction}
                onChange={(v) => filter({ direction: v })}
                options={[
                  { value: "", label: t("cashAll") },
                  { value: "income", label: t("cashIncome") },
                  { value: "expense", label: t("cashExpense") },
                ]}
              />
              <Select
                label={t("cashStatus")}
                value={status}
                onChange={(v) => filter({ status: v })}
                options={[
                  { value: "", label: t("cashAll") },
                  { value: "unmatched", label: t("cashUnmatched") },
                  { value: "tax", label: t("cashTaxStandalone") },
                  { value: "matched", label: t("cashMatched") },
                  { value: "cancelled", label: t("cashCancelled") },
                ]}
              />
            </form>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("cashDocument")}</TableHead>
                    <TableHead>{t("cashDate")}</TableHead>
                    <TableHead>{t("cashPartner")}</TableHead>
                    <TableHead>{t("cashVs")}</TableHead>
                    <TableHead>{t("cashAccount")}</TableHead>
                    <TableHead>{t("cashStatus")}</TableHead>
                    <TableHead className="text-right">
                      {t("cashAmount")}
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
                            m.cancelled
                              ? "cashCancelled"
                              : m.tax
                                ? "cashTaxBadge"
                                : m.matches.some((x) => !x.detached_at)
                                  ? "cashMatched"
                                  : "cashUnmatched",
                          )}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right whitespace-nowrap font-medium">
                        {moneyText(m.data.amount, language)}
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!result.items.length && (
              <p className="p-field text-muted">{t("cashEmpty")}</p>
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
      {form && (
        <CashForms
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
