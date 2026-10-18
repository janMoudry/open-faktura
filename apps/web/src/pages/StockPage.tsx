import { StockOverview } from "../components/organisms/StockOverview";
import { useEffect, useState, useRef } from "react";
import { Link, useParams } from "react-router";
import {
  Alert,
  Button,
  DatePicker,
  Field,
  Heading,
  Select,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  stockCards,
  stockMovements,
  stockExecute,
  listCatalogs,
  type StockCard,
  type StockPage as Page,
  type StockCommand,
  type Catalog,
  SalesFailure,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { apiUrl } from "../services/status";
type WithoutKey<T> = T extends unknown ? Omit<T, "request_id"> : never;
export function StockPage() {
  const { companyId = "" } = useParams(),
    { company, onExpired } = useWorkspace(),
    { t, language } = useAppTranslation();
  const [cards, setCards] = useState<StockCard[]>([]),
    [history, setHistory] = useState<Page | null>(null),
    [items, setItems] = useState<Catalog[]>([]),
    [query, setQuery] = useState(""),
    [selected, setSelected] = useState(""),
    [mode, setMode] = useState("open"),
    [quantity, setQuantity] = useState(""),
    [cost, setCost] = useState(""),
    [reason, setReason] = useState(""),
    [date, setDate] = useState(new Date().toLocaleDateString("sv-SE")),
    [from, setFrom] = useState(""),
    [to, setTo] = useState(""),
    [page, setPage] = useState(1),
    [attempt, setAttempt] = useState(0),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(false);
  const pending = useRef<{ input: string; key: string } | null>(null);
  const write = company?.id === companyId && company.role !== "viewer";
  useEffect(() => {
    const c = new AbortController();
    setError(false);
    setHistory(null);
    Promise.all([
      stockCards(apiUrl, companyId, c.signal),
      stockMovements(
        apiUrl,
        companyId,
        {
          ...(from ? { from } : {}),
          ...(to ? { to } : {}),
          ...(selected ? { catalog_id: selected } : {}),
          page: String(page),
        },
        c.signal,
      ),
      listCatalogs(
        apiUrl,
        companyId,
        { query, archived: false, page: 1, page_size: 100 },
        c.signal,
      ),
    ])
      .then(([a, h, i]) => {
        if (c.signal.aborted) return;
        setCards(a);
        setHistory(h);
        setItems(i.items);
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, query, selected, from, to, page, attempt]);
  const card = cards.find((c) => c.catalog_id === selected);
  useEffect(() => setMode(card ? "move" : "open"), [selected, !!card]);
  useEffect(() => setCost(""), [selected, mode]);
  const command = async (body: WithoutKey<StockCommand>) => {
    if (busy) return;
    const input = JSON.stringify(body);
    if (pending.current?.input !== input)
      pending.current = { input, key: crypto.randomUUID() };
    setBusy(true);
    setError(false);
    try {
      await stockExecute(apiUrl, companyId, {
        ...body,
        request_id: pending.current.key,
      } as StockCommand);
      pending.current = null;
      setAttempt((v) => v + 1);
      setQuantity("");
      setCost("");
      setReason("");
    } catch (e) {
      setError(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  };
  const quantityText = (v: string) =>
    language === "cs" ? v.replace(".", ",") : v;
  const kindKey = (v: string): AppKey =>
    v === "retail"
      ? "retailTitle"
      : v === "invoice"
        ? "stockInvoice"
        : v === "received"
          ? "stockReceived"
          : v === "credit"
            ? "stockCredit"
            : v === "opening"
              ? "stockOpen"
              : v === "count"
                ? "stockCount"
                : v === "reversal"
                  ? "stockReversal"
                  : "stockMove";
  return (
    <div className="space-y-group min-w-0">
      <Heading level={1} size="page">
        {t("stockTitle")}
      </Heading>
      <p className="text-muted">{t("stockIntro")}</p>
      <StockOverview companyId={companyId} key={companyId} refresh={attempt} />
      {error && (
        <Alert tone="danger">
          {t("stockFailed")}{" "}
          <Button onClick={() => setAttempt((v) => v + 1)}>
            {t("cashRetry")}
          </Button>
        </Alert>
      )}
      <section className="workspace-panel space-y-field p-field">
        <div className="flex flex-wrap items-end gap-field">
          <Field
            label={t("stockSearch")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <Select
            label={t("stockItem")}
            value={selected}
            onChange={(v) => {
              setSelected(v);
              setPage(1);
            }}
            options={[
              { value: "", label: t("stockAll") },
              ...items.map((i) => ({ value: i.id, label: i.data.name })),
            ]}
          />
        </div>
        {card && (
          <p>
            {t("stockBalance")}:{" "}
            <strong>
              {quantityText(card.quantity)} {card.item.unit}
            </strong>
          </p>
        )}
        {write && selected && (
          <form
            className="grid gap-field md:grid-cols-2 xl:grid-cols-4"
            onSubmit={(e) => {
              e.preventDefault();
              void command(
                card
                  ? mode === "count"
                    ? {
                        action: "count",
                        catalog_id: selected,
                        quantity,
                        ...(cost.trim()
                          ? {
                              acquisition_unit_cost: cost
                                .trim()
                                .replace(",", "."),
                            }
                          : {}),
                        revision: card.revision,
                        reason,
                      }
                    : {
                        action: "move",
                        catalog_id: selected,
                        quantity,
                        ...(cost.trim()
                          ? {
                              acquisition_unit_cost: cost
                                .trim()
                                .replace(",", "."),
                            }
                          : {}),
                        booked_on: date,
                        reason,
                      }
                  : {
                      action: "open",
                      catalog_id: selected,
                      quantity,
                      ...(cost.trim()
                        ? {
                            acquisition_unit_cost: cost
                              .trim()
                              .replace(",", "."),
                          }
                        : {}),
                      booked_on: date,
                      reason,
                    },
              );
            }}
          >
            {card && (
              <Select
                label={t("stockOperation")}
                value={mode}
                onChange={setMode}
                options={[
                  { value: "move", label: t("stockMove") },
                  { value: "count", label: t("stockCount") },
                ]}
              />
            )}
            <Field
              label={t("stockQuantity")}
              value={quantity}
              onChange={(e) => setQuantity(e.target.value)}
              inputMode="decimal"
              required
            />
            <Field
              label={t("stockManualCost")}
              value={cost}
              onChange={(e) => setCost(e.target.value)}
              inputMode="decimal"
              hint={t("stockManualCostHint")}
            />
            <DatePicker
              label={t("stockDate")}
              value={date}
              onChange={setDate}
              disabled={!!card && mode === "count"}
            />
            <Field
              label={t("stockReason")}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              maxLength={500}
              required
            />
            <p className="text-sm text-muted">
              {t(card && mode === "count" ? "stockCountHelp" : "stockMoveHelp")}
            </p>
            <Button
              type="submit"
              disabled={busy || !reason.trim() || !quantity}
            >
              {t(card ? "stockSave" : "stockOpen")}
            </Button>
          </form>
        )}
      </section>
      <section className="workspace-panel overflow-hidden">
        <div className="flex flex-wrap items-end gap-field p-field">
          <DatePicker
            label={t("stockFrom")}
            value={from}
            onChange={(v) => {
              setFrom(v);
              setPage(1);
            }}
          />
          <DatePicker
            label={t("stockTo")}
            value={to}
            onChange={(v) => {
              setTo(v);
              setPage(1);
            }}
          />
        </div>
        {history?.summary && card && (
          <section
            className="border-y border-border p-field space-y-field"
            aria-label={t("stockStatistics")}
          >
            <Heading level={2} size="section">
              {t("stockStatistics")}
            </Heading>
            <dl className="grid grid-cols-2 gap-field lg:grid-cols-5">
              {(
                [
                  "opening",
                  "received",
                  "issued",
                  "adjustments",
                  "closing",
                ] as const
              ).map((key) => (
                <div key={key}>
                  <dt className="text-sm text-muted">
                    {t(
                      (
                        {
                          opening: "stockStatOpening",
                          received: "stockStatReceived",
                          issued: "stockStatIssued",
                          adjustments: "stockStatAdjustments",
                          closing: "stockStatClosing",
                        } as const
                      )[key],
                    )}
                  </dt>
                  <dd className="font-semibold tabular-nums">
                    {quantityText(history.summary![key])} {card.item.unit}
                  </dd>
                </div>
              ))}
            </dl>
            <p className="text-sm text-muted">{t("stockStatHelp")}</p>
          </section>
        )}
        <div
          className="overflow-x-auto"
          role="region"
          aria-label={t("stockTitle")}
          tabIndex={0}
        >
          <Table className="whitespace-nowrap">
            <TableHeader>
              <TableRow>
                <TableHead>{t("stockDate")}</TableHead>
                <TableHead>{t("stockItem")}</TableHead>
                <TableHead>{t("stockQuantity")}</TableHead>
                <TableHead>{t("stockOperation")}</TableHead>
                <TableHead>{t("stockReason")}</TableHead>
                <TableHead>{t("inboxActions")}</TableHead>
              </TableRow>
            </TableHeader>
            <tbody>
              {history?.items.map((m) => (
                <TableRow key={m.id}>
                  <TableCell>
                    {new Date(m.booked_on + "T12:00:00").toLocaleDateString(
                      language,
                    )}
                  </TableCell>
                  <TableCell>{m.item.name}</TableCell>
                  <TableCell>
                    {quantityText(m.quantity)} {m.item.unit}
                  </TableCell>
                  <TableCell>{t(kindKey(m.kind))}</TableCell>
                  <TableCell>
                    {m.source_id && m.source_kind ? (
                      <Link
                        className="underline underline-offset-4"
                        to={`/companies/${companyId}/${({ invoice: "invoices", received: "received-invoices", credit: "credit-notes", retail: "retail" } as Record<string, string>)[m.source_kind]}/${m.source_id}`}
                      >
                        {t(kindKey(m.kind))} #{m.source_id}
                      </Link>
                    ) : (
                      m.reason
                    )}
                    {m.acquisition_unit_cost !== null && (
                      <p className="text-sm text-muted">
                        {t("stockAcquisitionCost")}:{" "}
                        {quantityText(m.acquisition_unit_cost)}
                      </p>
                    )}
                    {m.acquisition_extra_cost &&
                      m.acquisition_extra_cost !== "0" && (
                        <p className="text-sm text-muted">
                          {t("stockAllocatedCost")}:{" "}
                          {quantityText(m.acquisition_extra_cost)} CZK
                        </p>
                      )}
                    {m.acquisition_correction && (
                      <p className="text-sm text-muted">
                        {t("stockAcquisitionCorrection")}
                      </p>
                    )}
                  </TableCell>
                  <TableCell>
                    {write && ["manual", "count"].includes(m.kind) && (
                      <Button
                        variant="ghost"
                        disabled={busy || !reason.trim()}
                        onClick={() =>
                          command({
                            action: "reverse",
                            catalog_id: m.catalog_id,
                            movement_id: m.id,
                            reason,
                          })
                        }
                      >
                        {t("stockReversal")}
                      </Button>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </tbody>
          </Table>
        </div>
        {!history?.items.length && <p className="p-field">{t("stockEmpty")}</p>}
        <div className="flex justify-end gap-control p-field">
          <Button
            variant="secondary"
            disabled={page === 1}
            onClick={() => setPage((v) => v - 1)}
          >
            {t("retailPrevious")}
          </Button>
          <Button
            variant="secondary"
            disabled={!history || page * 25 >= history.total}
            onClick={() => setPage((v) => v + 1)}
          >
            {t("retailNext")}
          </Button>
        </div>
      </section>
    </div>
  );
}
