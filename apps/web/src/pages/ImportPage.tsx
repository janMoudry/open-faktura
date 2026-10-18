import { CatalogImportMapping } from "../components/organisms/CatalogImportMapping";
import {
  mapCatalogImport,
  suggestCatalogMapping,
  requiredCatalogFields,
} from "../utils/catalogImport";
import { useEffect, useRef, useState } from "react";
import { useParams } from "react-router";
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  Alert,
  Button,
  Checkbox,
  Field,
  Heading,
  Select,
} from "@open-faktura/ui";
import {
  applyImport,
  getImport,
  listImports,
  parseImportTable,
  prepareImport,
  prepareMoneyImport,
  InvoiceCalculationFailure,
  type ImportBatch,
  type ImportRecord,
  type ImportTable,
  type ImportPrepare,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { apiUrl } from "../services/status";
import {
  importFields,
  mapImportContacts,
  saveImportReport,
  suggestImportMapping,
  type ImportField,
} from "../utils/import";
import { ImportPreview } from "../components/organisms/ImportPreview";
export function ImportPage() {
  const { companyId = "" } = useParams();
  return <ImportWorkspace key={companyId} companyId={companyId} />;
}
function ImportWorkspace({ companyId }: { companyId: string }) {
  const { t } = useAppTranslation();
  const { company, onExpired } = useWorkspace();
  const editable = company?.id === companyId && company.role !== "viewer";
  const [source, setSource] = useState<ImportPrepare["source"]>("idoklad"),
    [agenda, setAgenda] = useState(""),
    [delimiter, setDelimiter] = useState<";" | "," | "tab">(";");
  const [table, setTable] = useState<ImportTable | null>(null),
    [records, setRecords] = useState<ImportRecord[] | null>(null),
    [mapping, setMapping] = useState<Record<ImportField, number>>(() =>
      suggestImportMapping([]),
    );
  const [batch, setBatch] = useState<ImportBatch | null>(null),
    [history, setHistory] = useState<ImportBatch[]>([]),
    [skip, setSkip] = useState<number[]>([]),
    [confirmed, setConfirmed] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<AppKey | "">(""),
    [filename, setFilename] = useState("");
  const [moneyCzk, setMoneyCzk] = useState(false);
  const [tableKind, setTableKind] = useState<"contact" | "catalog">("contact");
  const [catalogMapping, setCatalogMapping] = useState(() =>
    suggestCatalogMapping([]),
  );
  const [priceMode, setPriceMode] = useState<"" | "net" | "gross">("");
  const [decimalSeparator, setDecimalSeparator] = useState<"." | ",">(".");
  const catalogTable = source === "generic" && tableKind === "catalog";
  const request = useRef<{ body: string; id: string } | null>(null);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    const c = new AbortController();
    listImports(apiUrl, companyId, c.signal)
      .then(setHistory)
      .catch((e) => {
        if (!c.signal.aborted) failure(e);
      });
    return () => {
      mounted.current = false;
      c.abort();
    };
  }, [companyId]);
  function failure(e: unknown) {
    if (e instanceof InvoiceCalculationFailure && e.status === 401) onExpired();
    else
      setError(
        e instanceof InvoiceCalculationFailure && e.status === 403
          ? "companyForbidden"
          : e instanceof InvoiceCalculationFailure && e.status === 422
            ? "importInvalidFile"
            : e instanceof InvoiceCalculationFailure && e.status === 409
              ? "importConflict"
              : "requestFailed",
      );
  }
  async function run(action: () => Promise<void>) {
    setError("");
    setBusy(true);
    try {
      await action();
    } catch (e) {
      if (mounted.current) failure(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  async function load(file: File) {
    setTable(null);
    setRecords(null);
    setFilename("");
    await run(async () => {
      if (file.size > 2000000) throw new InvoiceCalculationFailure(422, "file");
      if (source === "money_s3") {
        if (
          !moneyCzk ||
          !agenda.trim() ||
          !file.name.toLowerCase().endsWith(".xml")
        )
          throw new InvoiceCalculationFailure(422, "file");
        const xml = new TextDecoder("utf-8", { fatal: true }).decode(
          await file.arrayBuffer(),
        );
        const payload = {
          xml,
          agenda: agenda.trim(),
          domestic_currency: "CZK" as const,
        };
        const body = JSON.stringify(payload);
        if (request.current?.body !== body)
          request.current = { body, id: crypto.randomUUID() };
        const value = await prepareMoneyImport(apiUrl, companyId, {
          ...payload,
          request_id: request.current.id,
        });
        if (mounted.current) {
          setBatch(value);
          setSkip([]);
          setConfirmed(false);
          setFilename(file.name);
          setHistory(await listImports(apiUrl, companyId));
        }
      } else if (file.name.toLowerCase().endsWith(".json")) {
        const value: unknown = JSON.parse(await file.text());
        if (!Array.isArray(value) || !value.length || value.length > 500)
          throw new InvoiceCalculationFailure(422, "file");
        if (mounted.current) {
          setRecords(value as ImportRecord[]);
          setFilename(file.name);
        }
      } else {
        const bytes = new Uint8Array(await file.arrayBuffer());
        let text = "";
        for (const b of bytes) text += String.fromCharCode(b);
        const result = await parseImportTable(apiUrl, companyId, {
          format: file.name.toLowerCase().endsWith(".xlsx") ? "xlsx" : "csv",
          content: btoa(text),
          delimiter,
        });
        if (mounted.current) {
          setTable(result);
          setMapping(suggestImportMapping(result.headers));
          setCatalogMapping(suggestCatalogMapping(result.headers));
          setFilename(file.name);
        }
      }
    });
  }
  async function preview() {
    await run(async () => {
      const payload = {
        source,
        agenda: agenda.trim(),
        records: table
          ? catalogTable
            ? mapCatalogImport(
                table,
                catalogMapping,
                priceMode as "net" | "gross",
                decimalSeparator,
              )
            : mapImportContacts(table, mapping)
          : records!,
      };
      const body = JSON.stringify(payload);
      if (request.current?.body !== body)
        request.current = { body, id: crypto.randomUUID() };
      const value = await prepareImport(apiUrl, companyId, {
        ...payload,
        request_id: request.current.id,
      });
      if (mounted.current) {
        setBatch(value);
        setSkip([]);
        setConfirmed(false);
        setHistory(await listImports(apiUrl, companyId));
      }
    });
  }
  async function commit() {
    if (!batch) return;
    await run(async () => {
      let value = await applyImport(apiUrl, companyId, batch.id, skip);
      if (!mounted.current) return;
      setBatch(value);
      setSkip([]);
      while (
        value.status !== "done" &&
        value.rows.some((r) => r.status === "ready") &&
        !value.rows.some((r) => ["invalid", "conflict"].includes(r.status))
      ) {
        value = await applyImport(apiUrl, companyId, value.id, []);
        if (!mounted.current) return;
        setBatch(value);
      }
      setHistory(await listImports(apiUrl, companyId));
      setConfirmed(false);
    });
  }
  const blocked = batch?.rows.some(
    (r) =>
      ["invalid", "conflict"].includes(r.status) && !skip.includes(r.index),
  );
  return (
    <div className="space-y-group">
      <Heading level={1} size="page">
        {t("navImport")}
      </Heading>
      <p className="text-muted">{t("importIntro")}</p>
      {batch?.source === "money_s3" && <Alert>{t("importGuideMoney")}</Alert>}
      {error && (
        <Alert tone="danger" role="alert">
          {t(error)}{" "}
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() =>
              void run(async () => {
                setHistory(await listImports(apiUrl, companyId));
                if (batch)
                  setBatch(await getImport(apiUrl, companyId, batch.id));
              })
            }
          >
            {t("retry")}
          </Button>
        </Alert>
      )}
      {batch ? (
        <>
          <div className="flex flex-wrap items-center justify-between gap-field">
            <p className="font-semibold">
              {t("importTarget", { name: company?.settings.name ?? "" })}
            </p>
            <div className="flex flex-wrap gap-control">
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() => saveImportReport(batch, batch.id)}
              >
                {t("importReport")}
              </Button>
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() => {
                  setBatch(null);
                  setConfirmed(false);
                }}
              >
                {t("importNew")}
              </Button>
            </div>
          </div>
          {batch.rows.some((row) => row.kind === "invoice") && (
            <Alert tone="warning">{t("importLimits")}</Alert>
          )}
          <ImportPreview
            batch={batch}
            company={companyId}
            skip={skip}
            onSkip={setSkip}
            disabled={busy || !editable}
          />
          {batch.status === "done" ? (
            <Alert tone="success" role="status">
              {t("importDone", {
                count: batch.rows.filter((r) => r.status === "imported").length,
              })}
            </Alert>
          ) : (
            editable && (
              <section className="workspace-panel p-field space-y-field">
                <p className="text-sm text-muted">{t("importBackup")}</p>
                <Checkbox
                  disabled={busy}
                  checked={confirmed}
                  onChange={(e) => setConfirmed(e.target.checked)}
                  label={t("importConfirm")}
                />
                <Button
                  loading={busy}
                  disabled={!confirmed || blocked}
                  onClick={() => void commit()}
                >
                  {t("importApply")}
                </Button>
                <p role="status" className="text-sm text-muted">
                  {t("importProgress", {
                    done: batch.rows.filter((r) =>
                      ["imported", "duplicate", "skipped"].includes(r.status),
                    ).length,
                    total: batch.rows.length,
                  })}
                </p>
              </section>
            )
          )}
        </>
      ) : (
        editable && (
          <section className="workspace-panel p-field space-y-group">
            <Heading level={2} size="card">
              {t("importChoose")}
            </Heading>
            <div className="grid gap-field md:grid-cols-2">
              <Select
                label={t("importSource")}
                value={source}
                disabled={busy}
                onChange={(v) => {
                  setSource(v as typeof source);
                  setRecords(null);
                  setTable(null);
                  setFilename("");
                  setMoneyCzk(false);
                }}
                options={[
                  { value: "idoklad", label: "iDoklad" },
                  { value: "fakturaonline", label: "FakturaOnline" },
                  { value: "generic", label: t("importGeneric") },
                  { value: "money_s3", label: "Money S3 (XML)" },
                ]}
              />
              <Field
                label={t("importAgenda")}
                hint={t("importAgendaHint")}
                value={agenda}
                maxLength={200}
                disabled={busy}
                onChange={(e) => setAgenda(e.target.value)}
              />
            </div>
            <Alert>
              {t(
                source === "money_s3"
                  ? "importGuideMoney"
                  : source === "idoklad"
                    ? "importGuideIdoklad"
                    : source === "fakturaonline"
                      ? "importGuideFaktura"
                      : "importGuideGeneric",
              )}{" "}
              {source !== "generic" && (
                <a
                  className="underline underline-offset-4"
                  href={
                    source === "money_s3"
                      ? "https://money.cz/navod/s3xmlde/"
                      : source === "idoklad"
                        ? "https://www.idoklad.cz/podpora/adresar-kontakty"
                        : "https://www.fakturaonline.cz/navody/pokrocile-funkce/statistiky-vystavenych-faktur"
                  }
                  target="_blank"
                  rel="noreferrer"
                >
                  {t("importHelp")}
                </a>
              )}
            </Alert>
            {source === "generic" && (
              <Select
                label={t("importTableKind")}
                value={tableKind}
                disabled={busy}
                onChange={(v) => {
                  setTableKind(v as "contact" | "catalog");
                  setTable(null);
                  setRecords(null);
                  setFilename("");
                }}
                options={[
                  { value: "contact", label: t("navContacts") },
                  { value: "catalog", label: t("importCatalog") },
                ]}
              />
            )}
            {catalogTable && (
              <>
                <Alert>{t("importCatalogGuide")}</Alert>
                <a
                  className="text-sm underline underline-offset-4"
                  href="/catalog-import.csv"
                  download
                >
                  {t("importCatalogTemplate")}
                </a>
                <div className="grid gap-field sm:grid-cols-2">
                  <Select
                    label={t("importCatalogPriceMode")}
                    required
                    value={priceMode}
                    disabled={busy}
                    onChange={(v) => setPriceMode(v as typeof priceMode)}
                    options={[
                      { value: "", label: t("importNotMapped") },
                      { value: "net", label: t("itemSalesNet") },
                      { value: "gross", label: t("itemSalesGross") },
                    ]}
                  />
                  <Select
                    label={t("importCatalogDecimal")}
                    value={decimalSeparator}
                    disabled={busy}
                    onChange={(v) => setDecimalSeparator(v as "." | ",")}
                    options={[
                      { value: ".", label: t("importCatalogDot") },
                      { value: ",", label: t("importCatalogComma") },
                    ]}
                  />
                </div>
              </>
            )}
            {source === "money_s3" ? (
              <Checkbox
                label={t("importMoneyCzk")}
                checked={moneyCzk}
                onChange={(e) => setMoneyCzk(e.target.checked)}
              />
            ) : (
              <p className="text-sm text-muted">{t("importFormats")}</p>
            )}
            {source !== "money_s3" && !catalogTable && (
              <a
                className="text-sm underline underline-offset-4"
                href="/import-format.json"
                download
              >
                {t("importTemplate")}
              </a>
            )}
            <div className="grid gap-field sm:grid-cols-2">
              {source !== "money_s3" && (
                <Select
                  label={t("importDelimiter")}
                  disabled={busy || !!table}
                  value={delimiter}
                  onChange={(v) => setDelimiter(v as typeof delimiter)}
                  options={[
                    { value: ";", label: t("importSemicolon") },
                    { value: ",", label: t("importComma") },
                    { value: "tab", label: t("importTab") },
                  ]}
                />
              )}
              <Field
                type="file"
                label={t("importFile")}
                accept={source === "money_s3" ? ".xml" : ".csv,.xlsx,.json"}
                disabled={
                  busy ||
                  (source === "money_s3" && (!moneyCzk || !agenda.trim()))
                }
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) void load(f);
                  e.target.value = "";
                }}
              />
            </div>
            {filename && (
              <p role="status" className="text-sm">
                {filename} · {table?.rows.length ?? records?.length}{" "}
                {t("importRows")}
              </p>
            )}
            {table && (
              <>
                <Heading level={3} size="card">
                  {t(catalogTable ? "importCatalogMapping" : "importMapping")}
                </Heading>
                <p className="text-sm text-muted">
                  {t(
                    catalogTable
                      ? "importCatalogMappingHint"
                      : "importMappingHint",
                  )}
                </p>
                {catalogTable ? (
                  <CatalogImportMapping
                    headers={table.headers}
                    mapping={catalogMapping}
                    onChange={setCatalogMapping}
                    disabled={busy}
                  />
                ) : (
                  <div className="grid gap-field sm:grid-cols-2 lg:grid-cols-3">
                    {importFields.map((k) => (
                      <Select
                        key={k}
                        label={t(`importField_${k}` as AppKey)}
                        required={["key", "name", "country"].includes(k)}
                        value={String(mapping[k])}
                        disabled={busy}
                        onChange={(v) =>
                          setMapping({ ...mapping, [k]: Number(v) })
                        }
                        options={[
                          { value: "-1", label: t("importNotMapped") },
                          ...table.headers.map((h, i) => ({
                            value: String(i),
                            label: h,
                          })),
                        ]}
                      />
                    ))}
                  </div>
                )}
                <div
                  className="overflow-x-auto"
                  tabIndex={0}
                  role="region"
                  aria-label={t("importMapping")}
                >
                  <Table className="min-w-invoice-table">
                    <TableHeader>
                      <TableRow>
                        {table.headers.map((h) => (
                          <TableHead key={h}>{h}</TableHead>
                        ))}
                      </TableRow>
                    </TableHeader>
                    <tbody>
                      {table.rows.slice(0, 3).map((r, i) => (
                        <TableRow key={i}>
                          {r.map((v, j) => (
                            <TableCell key={j}>{v}</TableCell>
                          ))}
                        </TableRow>
                      ))}
                    </tbody>
                  </Table>
                </div>
              </>
            )}
            <Button
              loading={busy}
              disabled={
                !agenda.trim() ||
                (!table && !records) ||
                (!!table &&
                  (catalogTable
                    ? !priceMode ||
                      requiredCatalogFields.some((k) => catalogMapping[k] < 0)
                    : ["key", "name", "country"].some(
                        (k) => mapping[k as ImportField] < 0,
                      )))
              }
              onClick={() => void preview()}
            >
              {t("importPreviewAction")}
            </Button>
          </section>
        )
      )}
      {!batch && (
        <section className="workspace-panel p-field space-y-field">
          <Heading level={2} size="card">
            {t("importHistory")}
          </Heading>
          {history.length === 0 ? (
            <p className="text-muted">{t("importEmpty")}</p>
          ) : (
            <ul className="space-y-control">
              {history.map((b) => (
                <li key={b.id}>
                  <Button
                    variant="secondary"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => {
                        setBatch(await getImport(apiUrl, companyId, b.id));
                        setSkip([]);
                        setConfirmed(false);
                      })
                    }
                  >
                    {b.source} · {b.agenda} ·{" "}
                    {t(b.status === "done" ? "importFinished" : "importResume")}
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </div>
  );
}
