import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Button,
  DatePicker,
  Field,
  Select,
  Heading,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  stockReport,
  stockCosts,
  stockSetCost,
  SalesFailure,
  type StockReport,
  type StockCost,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function StockOverview({
  companyId,
  refresh,
}: {
  companyId: string;
  refresh: number;
}) {
  const { t, language } = useAppTranslation();
  const { company, onExpired } = useWorkspace();
  const [date, setDate] = useState(new Date().toLocaleDateString("sv-SE")),
    [query, setQuery] = useState(""),
    [page, setPage] = useState(1),
    [attempt, setAttempt] = useState(0);
  const [report, setReport] = useState<StockReport | null>(null),
    [error, setError] = useState(false),
    [busy, setBusy] = useState(false);
  const [editor, setEditor] = useState<StockReport["items"][number] | null>(
      null,
    ),
    [price, setPrice] = useState(""),
    [effective, setEffective] = useState(date),
    [reason, setReason] = useState(""),
    [history, setHistory] = useState<StockCost[]>([]);
  const [valuation, setValuation] = useState("manual");
  const pending = useRef<{ input: string; key: string } | null>(null);
  useEffect(() => {
    const c = new AbortController();
    setReport(null);
    setError(false);
    stockReport(
      apiUrl,
      companyId,
      { as_of: date, query, page: String(page), valuation },
      c.signal,
    )
      .then((v) => {
        if (!c.signal.aborted) setReport(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, date, query, page, attempt, refresh, valuation]);
  useEffect(() => {
    setEditor(null);
  }, [companyId]);
  useEffect(() => {
    setHistory([]);
    if (!editor) return;
    const c = new AbortController();
    stockCosts(apiUrl, companyId, editor.catalog_id, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setHistory(v);
      })
      .catch(() => {
        if (!c.signal.aborted) setError(true);
      });
    return () => c.abort();
  }, [companyId, editor]);
  const quantity = (v: string) => (language === "cs" ? v.replace(".", ",") : v);
  const save = async () => {
    if (!editor || busy) return;
    const body = {
      catalog_id: editor.catalog_id,
      revision: editor.cost_revision,
      effective_on: effective,
      unit_cost: price.trim() ? price.replace(",", ".") : null,
      reason,
    };
    const input = JSON.stringify(body);
    if (pending.current?.input !== input)
      pending.current = { input, key: crypto.randomUUID() };
    setBusy(true);
    setError(false);
    try {
      await stockSetCost(apiUrl, companyId, {
        ...body,
        request_id: pending.current.key,
      });
      pending.current = null;
      setEditor(null);
      setAttempt((v) => v + 1);
    } catch (e) {
      setError(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  };
  return (
    <section
      className="workspace-panel space-y-field p-field"
      aria-label={t("stockOverviewTitle")}
    >
      <Heading level={2} size="section">
        {t("stockOverviewTitle")}
      </Heading>
      <p className="text-sm text-muted">
        {t(
          valuation === "manual" ? "stockValuationHelp" : "stockAutomaticHelp",
        )}
      </p>
      <div className="flex flex-wrap items-end gap-field">
        <Select
          label={t("stockValuationMethod")}
          value={valuation}
          options={[
            { value: "manual", label: t("stockManualMethod") },
            { value: "automatic", label: t("stockAutomaticMethod") },
          ]}
          onChange={(v) => {
            setValuation(v);
            setPage(1);
            setEditor(null);
          }}
        />
        <DatePicker
          label={t("stockAsOf")}
          value={date}
          onChange={(v) => {
            setDate(v);
            setPage(1);
          }}
        />
        <Field
          label={t("stockOverviewSearch")}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setPage(1);
          }}
        />
      </div>
      {error && (
        <Alert tone="danger">
          {t("stockValuationFailed")}{" "}
          <Button
            variant="secondary"
            onClick={() => {
              setEditor(null);
              setAttempt((v) => v + 1);
            }}
          >
            {t("cashRetry")}
          </Button>
        </Alert>
      )}
      {!report && !error && <p role="status">{t("stockOverviewLoading")}</p>}
      {report && (
        <>
          <dl className="flex flex-wrap gap-group">
            <div>
              <dt className="text-muted">{t("stockKnownValue")}</dt>
              <dd className="font-semibold tabular-nums">
                {moneyText(report.known_value, language)}
              </dd>
            </div>
            <div>
              <dt className="text-muted">{t("stockUnvalued")}</dt>
              <dd>{report.unvalued_count}</dd>
            </div>
            <div>
              <dt className="text-muted">{t("stockNegative")}</dt>
              <dd>{report.negative_count}</dd>
            </div>
          </dl>
          {report.unvalued_count > 0 && (
            <Alert tone="warning">{t("stockPartialValue")}</Alert>
          )}
          <div
            className="overflow-x-auto"
            role="region"
            aria-label={t("stockOverviewTable")}
            tabIndex={0}
          >
            <Table className="whitespace-nowrap">
              <TableHeader>
                <TableRow>
                  {(
                    [
                      "stockItem",
                      "stockCode",
                      "stockQuantity",
                      "stockUnitCost",
                      "stockValue",
                      "inboxActions",
                    ] as const
                  )
                    .filter(
                      (k) => valuation === "manual" || k !== "inboxActions",
                    )
                    .map((k) => (
                      <TableHead key={k}>{t(k)}</TableHead>
                    ))}
                  {valuation === "automatic" && (
                    <>
                      <TableHead>{t("stockIssuedCost")}</TableHead>
                      <TableHead>{t("stockCorrectionStock")}</TableHead>
                      <TableHead>{t("stockCorrectionIssued")}</TableHead>
                    </>
                  )}
                </TableRow>
              </TableHeader>
              <tbody>
                {report.items.map((i) => (
                  <TableRow key={i.catalog_id}>
                    <TableCell>
                      {i.name}
                      {i.archived && (
                        <span className="text-muted">
                          {" "}
                          — {t("stockArchived")}
                        </span>
                      )}
                    </TableCell>
                    <TableCell>{i.code || "—"}</TableCell>
                    <TableCell className="text-right tabular-nums">
                      {quantity(i.quantity)} {i.unit}
                    </TableCell>
                    <TableCell className="text-right tabular-nums">
                      {i.unit_cost === null
                        ? t("stockUnknown")
                        : quantity(i.unit_cost) + " CZK"}
                    </TableCell>
                    <TableCell className="text-right tabular-nums">
                      {i.value === null
                        ? t("stockUnknown")
                        : moneyText(i.value, language)}
                    </TableCell>
                    {valuation === "manual" && (
                      <TableCell>
                        <Button
                          variant="ghost"
                          onClick={() => {
                            setEditor(i);
                            setPrice(i.unit_cost ?? "");
                            setEffective(date);
                            setReason("");
                          }}
                        >
                          {t("stockValuationDetail")}
                        </Button>
                      </TableCell>
                    )}
                    {valuation === "automatic" &&
                      [
                        i.issued_value,
                        i.correction_stock_value,
                        i.correction_issued_value,
                      ].map((v, n) => (
                        <TableCell key={n} className="text-right tabular-nums">
                          {v === null
                            ? t("stockUnknown")
                            : moneyText(v, language)}
                        </TableCell>
                      ))}
                  </TableRow>
                ))}
              </tbody>
            </Table>
          </div>
          {!report.items.length && <p>{t("stockOverviewEmpty")}</p>}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={page === 1}
              onClick={() => setPage((v) => v - 1)}
            >
              {t("retailPrevious")}
            </Button>
            <Button
              variant="secondary"
              disabled={page * 25 >= report.total}
              onClick={() => setPage((v) => v + 1)}
            >
              {t("retailNext")}
            </Button>
          </div>
        </>
      )}
      {editor && (
        <section
          className="border-t border-border pt-field space-y-field"
          aria-label={t("stockValuationDetail")}
        >
          <Heading level={3} size="card">
            {editor.name}
          </Heading>
          {company?.id === companyId && company.role !== "viewer" && (
            <form
              className="flex flex-wrap items-end gap-field"
              onSubmit={(e) => {
                e.preventDefault();
                void save();
              }}
            >
              <Field
                label={t("stockUnitCost")}
                value={price}
                inputMode="decimal"
                onChange={(e) => setPrice(e.target.value)}
                disabled={busy}
              />
              <DatePicker
                label={t("stockCostEffective")}
                value={effective}
                onChange={setEffective}
                disabled={busy}
              />
              <Field
                label={t("stockCostReason")}
                value={reason}
                maxLength={500}
                required
                onChange={(e) => setReason(e.target.value)}
                disabled={busy}
              />
              <Button type="submit" disabled={busy || !reason.trim()}>
                {t("stockSaveCost")}
              </Button>
            </form>
          )}
          <p className="text-sm text-muted">{t("stockCostHistoryHelp")}</p>
          <ul className="space-y-control">
            {history.map((h) => (
              <li key={h.revision}>
                <span className="font-medium">
                  {new Date(h.effective_on + "T12:00:00").toLocaleDateString(
                    language,
                  )}{" "}
                  —{" "}
                  {h.unit_cost === null
                    ? t("stockUnknown")
                    : quantity(h.unit_cost) + " CZK"}
                </span>{" "}
                · {h.reason}
              </li>
            ))}
          </ul>
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => setEditor(null)}
          >
            {t("stockCloseCost")}
          </Button>
        </section>
      )}
    </section>
  );
}
