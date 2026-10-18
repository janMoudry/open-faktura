import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import {
  Alert,
  Button,
  Dialog,
  Field,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
  Checkbox,
} from "@open-faktura/ui";
import {
  settlementCandidates,
  configureSettlement,
  getSales,
  InvoiceCalculationFailure,
  SalesFailure,
  type SalesDocument,
  type InvoiceDraft,
  type SettlementCandidates,
} from "@open-faktura/api-client";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
import { invoiceError } from "../../utils/invoice";
import { moneyText } from "../../utils/invoice";
export function AdvanceSettlementPicker({
  company,
  initial,
  invoice,
  onClose,
  onSaved,
}: {
  company: string;
  initial?: SalesDocument;
  invoice?: InvoiceDraft;
  onClose: () => void;
  onSaved?: () => void;
}) {
  const { t, language } = useAppTranslation(),
    { onExpired } = useWorkspace(),
    navigate = useNavigate();
  const [result, setResult] = useState<SettlementCandidates | null>(null),
    [selected, setSelected] = useState<Record<string, SalesDocument>>(
      initial ? { [initial.id]: initial } : {},
    ),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<AppKey | "">(""),
    [page, setPage] = useState(1),
    [query, setQuery] = useState(
      initial?.data.invoice.customer?.name ??
        invoice?.data.customer?.name ??
        "",
    ),
    [loaded, setLoaded] = useState(!invoice?.advances);
  const [pinned, setPinned] = useState<SalesDocument[]>(
    initial ? [initial] : [],
  );
  const key = useRef(crypto.randomUUID()),
    last = useRef("");
  function failure(e: unknown) {
    if (
      (e instanceof InvoiceCalculationFailure || e instanceof SalesFailure) &&
      e.status === 401
    )
      onExpired();
    else if (
      e instanceof InvoiceCalculationFailure &&
      e.fields.includes("advance_tax_exchange_rate")
    )
      setError("settlementFxTaxUnsupported");
    else if (
      e instanceof InvoiceCalculationFailure &&
      e.fields.some((f) => f.startsWith("advance_"))
    )
      setError("settlementInvalid");
    else setError(invoiceError(e));
  }
  useEffect(() => {
    let active = true;
    const sources = invoice?.advances?.sources ?? [];
    if (sources.length)
      Promise.all(sources.map((s) => getSales(apiUrl, company, s.id)))
        .then((values) => {
          if (active) {
            setSelected(Object.fromEntries(values.map((v) => [v.id, v])));
            setPinned(values);
            setLoaded(true);
          }
        })
        .catch((e) => {
          if (active) failure(e);
        });
    return () => {
      active = false;
    };
  }, [company, invoice?.id]);
  useEffect(() => {
    const c = new AbortController();
    setResult(null);
    settlementCandidates(apiUrl, company, query, page, c.signal)
      .then((v) => {
        if (!c.signal.aborted) setResult(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) failure(e);
      });
    return () => c.abort();
  }, [company, query, page]);
  async function save() {
    setBusy(true);
    setError("");
    try {
      const fields = {
        invoice_id: invoice?.id ?? null,
        revision: invoice?.revision ?? null,
        advances: Object.values(selected).map((v) => ({
          id: v.id,
          revision: v.revision,
        })),
      };
      const hash = JSON.stringify(fields);
      if (last.current && last.current !== hash)
        key.current = crypto.randomUUID();
      last.current = hash;
      const d = await configureSettlement(apiUrl, company, {
        ...fields,
        request_id: key.current,
      });
      onClose();
      if (invoice) onSaved?.();
      else navigate(`/companies/${company}/invoices/${d.id}`);
    } catch (e) {
      failure(e);
    } finally {
      setBusy(false);
    }
  }
  const values = Object.values(
    Object.fromEntries(
      [...pinned, ...(result?.items ?? []), ...Object.values(selected)].map(
        (v) => [v.id, v],
      ),
    ),
  );
  return (
    <Dialog
      open
      onClose={() => {
        if (!busy) onClose();
      }}
      title={t("settlementTitle")}
      description={t("settlementHelp")}
    >
      <div className="space-y-field">
        <p className="text-sm text-muted">{t("settlementCurrencyHelp")}</p>
        {error && (
          <Alert tone="danger" role="alert">
            {t(error)}
          </Alert>
        )}
        <Field
          label={t("settlementSearch")}
          value={query}
          disabled={busy}
          onChange={(e) => {
            setQuery(e.target.value);
            setPage(1);
          }}
        />
        {!result ? (
          <p role="status">{t("salesLoading")}</p>
        ) : (
          <>
            <div className="overflow-x-auto">
              <Table className="whitespace-nowrap">
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("settlementSelect")}</TableHead>
                    <TableHead>{t("taxAdvance")}</TableHead>
                    <TableHead>{t("draftCustomer")}</TableHead>
                    <TableHead>{t("taxReceived")}</TableHead>
                  </TableRow>
                </TableHeader>
                <tbody>
                  {values.map((v) => (
                    <TableRow key={v.id}>
                      <TableCell>
                        <Checkbox
                          label={v.number ?? v.data.name}
                          checked={!!selected[v.id]}
                          disabled={busy || !loaded}
                          onChange={(e) =>
                            setSelected((old) => {
                              const next = { ...old };
                              if (e.target.checked) next[v.id] = v;
                              else delete next[v.id];
                              return next;
                            })
                          }
                        />
                      </TableCell>
                      <TableCell>{v.number}</TableCell>
                      <TableCell>{v.data.invoice.customer?.name}</TableCell>
                      <TableCell>
                        {moneyText(
                          v.advance_paid ?? "0",
                          language,
                          v.data.invoice.calculation.currency,
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {!values.length && <p>{t("settlementEmpty")}</p>}
            <div className="flex gap-inline">
              <Button
                variant="ghost"
                disabled={busy || page === 1}
                onClick={() => setPage((p) => p - 1)}
              >
                {t("contactPrevious")}
              </Button>
              <Button
                variant="ghost"
                disabled={busy || page * 50 >= result.total}
                onClick={() => setPage((p) => p + 1)}
              >
                {t("contactNext")}
              </Button>
            </div>
          </>
        )}
        <p className="text-sm text-muted">
          {t("settlementSelection", { count: Object.keys(selected).length })}
        </p>
        <Button
          disabled={
            busy || !loaded || (!invoice && !Object.keys(selected).length)
          }
          onClick={() => void save()}
        >
          {t(invoice ? "settlementApply" : "settlementCreate")}
        </Button>
      </div>
    </Dialog>
  );
}
