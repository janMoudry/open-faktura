import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import {
  Alert,
  Button,
  DatePicker,
  Field,
  Heading,
  Select,
  Checkbox,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  itemSalesReport,
  itemSalesExport,
  SalesFailure,
  type ItemSalesReport,
  type ItemSalesSearch,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { apiUrl } from "../services/status";
export function ItemSalesPage() {
  const { companyId = "" } = useParams();
  return <ItemSalesContent key={companyId} companyId={companyId} />;
}
function ItemSalesContent({ companyId }: { companyId: string }) {
  const { t, language } = useAppTranslation();
  const { onExpired } = useWorkspace();
  const [from, setFrom] = useState(""),
    [to, setTo] = useState(""),
    [kind, setKind] = useState(""),
    [query, setQuery] = useState(""),
    [selected, setSelected] = useState(""),
    [page, setPage] = useState(1),
    [sourcePage, setSourcePage] = useState(1),
    [refresh, setRefresh] = useState(0);
  const [report, setReport] = useState<ItemSalesReport | null>(null),
    [error, setError] = useState(false),
    [exporting, setExporting] = useState(false),
    [settings, setSettings] = useState(false);
  const [columns, setColumns] = useState({
    code: true,
    net: true,
    gross: true,
  });
  useEffect(() => {
    try {
      const v = JSON.parse(
        localStorage.getItem("of.item-sales.columns") ?? "null",
      );
      if (v && [v.code, v.net, v.gross].every((x) => typeof x === "boolean"))
        setColumns(v);
    } catch {
      /* Optional browser preference. */
    }
  }, []);
  const filters: Record<string, string> = {
    ...(from ? { from } : {}),
    ...(to ? { to } : {}),
    ...(kind ? { kind } : {}),
    ...(selected ? { catalog_id: selected } : {}),
    query,
    page: String(page),
    source_page: String(sourcePage),
  };
  useEffect(() => {
    const controller = new AbortController();
    setReport(null);
    setError(false);
    itemSalesReport(apiUrl, companyId, filters, controller.signal)
      .then((r) => {
        if (!controller.signal.aborted) setReport(r);
      })
      .catch((e) => {
        if (!controller.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => controller.abort();
  }, [companyId, from, to, kind, selected, query, page, sourcePage, refresh]);
  const resetPages = () => {
    setPage(1);
    setSourcePage(1);
  };
  const change = (fn: (v: string) => void) => (v: string) => {
    fn(v);
    resetPages();
  };
  const decimal = (v: string) => (language === "cs" ? v.replace(".", ",") : v);
  const download = async (format: "csv" | "xlsx") => {
    setExporting(true);
    setError(false);
    try {
      const search: ItemSalesSearch = {
        ...(from ? { from } : {}),
        ...(to ? { to } : {}),
        ...(kind ? { kind: kind as ItemSalesSearch["kind"] } : {}),
        ...(selected ? { catalog_id: selected } : {}),
        query,
      };
      const blob = await itemSalesExport(apiUrl, companyId, {
        search,
        format,
        language,
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `item-sales.${format}`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) {
      setError(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setExporting(false);
    }
  };
  const typeName = (k: string) =>
    t(
      k === "invoice"
        ? "navInvoices"
        : k === "retail"
          ? "retailTitle"
          : "navCreditNotes",
    );
  return (
    <div className="space-y-group min-w-0">
      <Heading level={1} size="page">
        {t("itemSalesTitle")}
      </Heading>
      <p className="text-muted">{t("itemSalesHelp")}</p>
      <section className="workspace-panel space-y-field p-field">
        <div className="flex flex-wrap items-end gap-field">
          <DatePicker
            label={t("itemSalesFrom")}
            value={from}
            onChange={change(setFrom)}
          />
          <DatePicker
            label={t("itemSalesTo")}
            value={to}
            onChange={change(setTo)}
          />
          <Select
            label={t("itemSalesKind")}
            value={kind}
            onChange={change(setKind)}
            options={[
              { value: "", label: t("stockAll") },
              ...(["invoice", "retail", "credit_note"] as const).map((k) => ({
                value: k,
                label: typeName(k),
              })),
            ]}
          />
          <Field
            label={t("stockSearch")}
            value={query}
            onChange={(e) => change(setQuery)(e.target.value)}
          />
          <Button
            variant="secondary"
            onClick={() => {
              setFrom("");
              setTo("");
              setKind("");
              setQuery("");
              setSelected("");
              resetPages();
            }}
          >
            {t("itemSalesClear")}
          </Button>
          <Button
            variant="secondary"
            aria-expanded={settings}
            onClick={() => setSettings(!settings)}
          >
            {t("itemSalesColumns")}
          </Button>
          <Button
            variant="secondary"
            disabled={exporting || !report}
            onClick={() => void download("csv")}
          >
            CSV
          </Button>
          <Button
            variant="secondary"
            disabled={exporting || !report}
            onClick={() => void download("xlsx")}
          >
            XLSX
          </Button>
        </div>
        {settings && (
          <div className="flex flex-wrap gap-field">
            {(["code", "net", "gross"] as const).map((k) => (
              <Checkbox
                key={k}
                label={t(
                  k === "code"
                    ? "itemSalesCode"
                    : k === "net"
                      ? "itemSalesNet"
                      : "itemSalesGross",
                )}
                checked={columns[k]}
                onChange={(e) => {
                  const v = { ...columns, [k]: e.target.checked };
                  setColumns(v);
                  try {
                    localStorage.setItem(
                      "of.item-sales.columns",
                      JSON.stringify(v),
                    );
                  } catch {
                    /* Optional preference. */
                  }
                }}
              />
            ))}
          </div>
        )}
      </section>
      {error && (
        <Alert tone="danger">
          {t("itemSalesFailed")}{" "}
          <Button variant="secondary" onClick={() => setRefresh((x) => x + 1)}>
            {t("cashRetry")}
          </Button>
        </Alert>
      )}
      {!report && !error && <p role="status">{t("itemSalesLoading")}</p>}
      {report && (
        <>
          <section
            className="workspace-panel space-y-field p-field"
            aria-label={t("itemSalesTitle")}
          >
            <div className="flex flex-wrap gap-group">
              {report.totals.map((v) => (
                <p key={v.currency}>
                  {t("itemSalesTotal")}:{" "}
                  <strong>
                    {decimal(v.net)} / {decimal(v.gross)} {v.currency}
                  </strong>
                </p>
              ))}
            </div>
            {selected && (
              <Button
                variant="secondary"
                onClick={() => {
                  setSelected("");
                  resetPages();
                }}
              >
                {t("itemSalesAllItems")}
              </Button>
            )}
            <div className="overflow-x-auto">
              <Table className="whitespace-nowrap">
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("itemSalesItem")}</TableHead>
                    {columns.code && (
                      <TableHead>{t("itemSalesCode")}</TableHead>
                    )}
                    <TableHead>{t("itemSalesQuantity")}</TableHead>
                    {columns.net && <TableHead>{t("itemSalesNet")}</TableHead>}
                    {columns.gross && (
                      <TableHead>{t("itemSalesGross")}</TableHead>
                    )}
                  </TableRow>
                </TableHeader>
                <tbody>
                  {report.items.map((i) => (
                    <TableRow key={`${i.catalog_id}:${i.unit}:${i.currency}`}>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => {
                            setSelected(i.catalog_id);
                            resetPages();
                          }}
                        >
                          {i.name}
                        </Button>
                      </TableCell>
                      {columns.code && <TableCell>{i.code}</TableCell>}
                      <TableCell className="text-right tabular-nums">
                        {decimal(i.quantity)} {i.unit}
                      </TableCell>
                      {columns.net && (
                        <TableCell className="text-right tabular-nums">
                          {decimal(i.net)} {i.currency}
                        </TableCell>
                      )}
                      {columns.gross && (
                        <TableCell className="text-right tabular-nums">
                          {decimal(i.gross)} {i.currency}
                        </TableCell>
                      )}
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {report.total === 0 && <p>{t("itemSalesEmpty")}</p>}
            <div className="flex justify-end gap-inline">
              <Button
                variant="secondary"
                disabled={page === 1}
                onClick={() => setPage(page - 1)}
              >
                {t("retailPrevious")}
              </Button>
              <Button
                variant="secondary"
                disabled={page * 25 >= report.total}
                onClick={() => setPage(page + 1)}
              >
                {t("retailNext")}
              </Button>
            </div>
          </section>
          {selected && (
            <section
              className="workspace-panel space-y-field p-field"
              aria-label={t("itemSalesSources")}
            >
              <Heading level={2}>{t("itemSalesSources")}</Heading>
              <div className="overflow-x-auto">
                <Table className="whitespace-nowrap">
                  <TableHeader>
                    <TableRow>
                      {(
                        [
                          "itemSalesDocument",
                          "itemSalesDate",
                          "itemSalesQuantity",
                          "itemSalesNet",
                          "itemSalesGross",
                        ] as const
                      ).map((k) => (
                        <TableHead key={k}>{t(k)}</TableHead>
                      ))}
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {report.sources.map((i, index) => (
                      <TableRow key={`${i.kind}:${i.document_id}:${index}`}>
                        <TableCell>
                          <Link
                            className="underline"
                            to={`/companies/${companyId}/${i.kind === "invoice" ? "invoices" : i.kind === "retail" ? "retail" : "credit-notes"}/${i.document_id}`}
                          >
                            {i.number}
                          </Link>
                          <p className="text-muted text-sm">{i.description}</p>
                        </TableCell>
                        <TableCell>
                          {new Date(
                            i.issued_on + "T12:00:00",
                          ).toLocaleDateString(
                            language === "cs" ? "cs-CZ" : "en-GB",
                          )}
                        </TableCell>
                        <TableCell>
                          {decimal(i.quantity)} {i.unit}
                        </TableCell>
                        <TableCell>
                          {decimal(i.net)} {i.currency}
                        </TableCell>
                        <TableCell>
                          {decimal(i.gross)} {i.currency}
                        </TableCell>
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
              <div className="flex justify-end gap-inline">
                <Button
                  variant="secondary"
                  disabled={sourcePage === 1}
                  onClick={() => setSourcePage(sourcePage - 1)}
                >
                  {t("retailPrevious")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={sourcePage * 25 >= report.source_total}
                  onClick={() => setSourcePage(sourcePage + 1)}
                >
                  {t("retailNext")}
                </Button>
              </div>
            </section>
          )}
        </>
      )}
    </div>
  );
}
