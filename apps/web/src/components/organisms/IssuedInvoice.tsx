import { DocumentExports } from "./DocumentExports";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { DocumentOutput } from "./DocumentOutput";
import { DocumentTags } from "./DocumentTags";
import { InvoiceCredits } from "./InvoiceCredits";
import { AdvanceDeductions } from "./AdvanceDeductions";
import { DocumentAttachments } from "./DocumentAttachments";
import { DocumentDelivery } from "./DocumentDelivery";
import { InvoiceSummary } from "./InvoiceSummary";
import { PaymentStatus } from "../molecules/PaymentStatus";
import { InvoicePayments } from "./InvoicePayments";
import { useState } from "react";
import { Link, useSearchParams } from "react-router";
import {
  Tabs,
  Button,
  Icon,
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  Alert,
  Badge,
  Heading,
  formatCalendarDate,
} from "@open-faktura/ui";
import { invoiceIsdoc, type InvoiceDraft } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function IssuedInvoice({
  invoice,
  canCopy,
}: {
  invoice: InvoiceDraft;
  canCopy: boolean;
}) {
  const { t, language } = useAppTranslation();
  const { company } = useWorkspace();
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState(false);
  async function downloadIsdoc() {
    setExporting(true);
    setExportError(false);
    try {
      const file = await invoiceIsdoc(apiUrl, invoice.company_id, invoice.id);
      const url = URL.createObjectURL(file);
      const link = document.createElement("a");
      link.href = url;
      link.download = `invoice-${invoice.id}.isdoc`;
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch {
      setExportError(true);
    } finally {
      setExporting(false);
    }
  }
  const [params, setParams] = useSearchParams();
  const [settlement, setSettlement] = useState(invoice.settlement);
  const d = invoice.data,
    totals = invoice.totals!;
  const historical = d.calculation.calculation_version === "import-v1";
  const tabs = (
    <Tabs
      value={
        ["payments", "attachments"].includes(params.get("tab") ?? "")
          ? params.get("tab")!
          : "document"
      }
      onChange={(tab) =>
        setParams(
          (previous) => {
            const next = new URLSearchParams(previous);
            next.set("tab", tab);
            return next;
          },
          { replace: true },
        )
      }
      label={t("invoiceDetailTitle", { number: invoice.number ?? "" })}
      items={[
        {
          id: "document",
          label: t("invoiceDocumentTab"),
          content: (
            <div className="invoice-sheet">
              <section className="invoice-section">
                <dl className="invoice-facts max-w-3xl mx-auto">
                  <div>
                    <dt>{t("draftCustomer")}</dt>
                    <dd className="font-semibold">{d.customer?.name || "—"}</dd>
                  </div>
                  <div>
                    <dt>{t("draftOrder")}</dt>
                    <dd>{d.order_number || "—"}</dd>
                  </div>
                  <div>
                    <dt>{t("invoiceDescription")}</dt>
                    <dd className="whitespace-pre-wrap">
                      {d.note || d.calculation.lines[0]?.description || "—"}
                    </dd>
                  </div>
                </dl>
              </section>
              <section className="invoice-section bg-sidebar grid gap-section md:grid-cols-2">
                <div className="space-y-field">
                  <Heading level={2} size="card">
                    {t("invoiceDates")}
                  </Heading>
                  <dl className="invoice-facts">
                    {(
                      [
                        ["draftIssued", d.issued_on],
                        ["draftDue", d.due_on],
                        ["draftTaxDate", d.tax_on],
                      ] as const
                    ).map(
                      ([key, date]) =>
                        date && (
                          <div key={key}>
                            <dt>{t(key)}</dt>
                            <dd>{formatCalendarDate(date, language)}</dd>
                          </div>
                        ),
                    )}
                  </dl>
                </div>
                <div className="space-y-field">
                  <Heading level={2} size="card">
                    {t("invoicePaymentDetails")}
                  </Heading>
                  <dl className="invoice-facts">
                    <div>
                      <dt>{t("paymentMethod")}</dt>
                      <dd>
                        {t(
                          d.payment.method === "cash" ? "cash" : "bankTransfer",
                        )}
                      </dd>
                    </div>
                    <div>
                      <dt>{t("contactBankAccount")}</dt>
                      <dd>{d.payment.bank_account || "—"}</dd>
                    </div>
                    <div>
                      <dt>IBAN</dt>
                      <dd>{d.payment.iban || "—"}</dd>
                    </div>
                    <div>
                      <dt>{t("draftVariableSymbol")}</dt>
                      <dd>{d.payment.variable_symbol || "—"}</dd>
                    </div>
                  </dl>
                </div>
              </section>
              <details className="invoice-section text-sm">
                <summary className="cursor-pointer font-medium">
                  {t("invoiceMoreDetails")}
                </summary>
                <div className="grid gap-group sm:grid-cols-2 pt-field">
                  {(
                    [
                      ["draftSupplier", d.supplier],
                      ["draftCustomer", d.customer],
                    ] as const
                  ).map(
                    ([label, p]) =>
                      p && (
                        <div key={label} className="space-y-inline">
                          <Heading level={2} size="card">
                            {t(label)}
                          </Heading>
                          <p className="font-semibold">{p.name}</p>
                          <p>
                            {p.address.street}
                            <br />
                            {p.address.postal_code} {p.address.city},{" "}
                            {p.address.country}
                          </p>
                          {p.registration_number && (
                            <p>
                              {t("registrationNumber")}: {p.registration_number}
                            </p>
                          )}
                          {p.vat_number && (
                            <p>
                              {t("vatNumber")}: {p.vat_number}
                            </p>
                          )}
                          {p.email && <p>{p.email}</p>}
                          {p.phone && <p>{p.phone}</p>}
                        </div>
                      ),
                  )}
                </div>
                <p className="mt-field text-muted">{t("invoiceImmutable")}</p>
              </details>
              <section className="invoice-section">
                <Heading level={2} size="card">
                  {t("draftItems")}
                </Heading>
                <p className="text-sm text-muted">
                  {t(
                    d.calculation.price_mode === "net"
                      ? "draftNetPrice"
                      : "draftGrossPrice",
                  )}
                </p>
                <div
                  className="overflow-x-auto"
                  tabIndex={0}
                  role="region"
                  aria-label={t("draftItems")}
                >
                  <Table className="min-w-invoice-table">
                    <TableHeader>
                      <TableRow>
                        {(
                          [
                            "draftDescription",
                            "draftUnitPrice",
                            "draftQuantity",
                            "draftUnit",
                            ...(d.calculation.lines.some((line) =>
                              /[1-9]/.test(line.discount_percent),
                            )
                              ? ["draftDiscount" as const]
                              : []),
                            ...(d.calculation.tax_mode === "vat_payer"
                              ? ["draftVat" as const]
                              : []),
                            "draftTotal",
                          ] as const
                        ).map((k) => (
                          <TableHead
                            className={
                              k === "draftDescription" ? "" : "text-right"
                            }
                            key={k}
                          >
                            {t(k)}
                          </TableHead>
                        ))}
                      </TableRow>
                    </TableHeader>
                    <tbody>
                      {d.calculation.lines.map((l, i) => (
                        <TableRow key={l.id}>
                          <TableCell className="max-w-xs break-words">
                            {l.description}
                          </TableCell>
                          <TableCell className="text-right whitespace-nowrap tabular-nums">
                            {moneyText(l.unit_price, language, totals.currency)}
                          </TableCell>
                          <TableCell className="text-right whitespace-nowrap tabular-nums">
                            {l.quantity}
                          </TableCell>
                          <TableCell className="text-right whitespace-nowrap">
                            {l.unit || "—"}
                          </TableCell>
                          {d.calculation.lines.some((line) =>
                            /[1-9]/.test(line.discount_percent),
                          ) && (
                            <TableCell className="text-right whitespace-nowrap tabular-nums">
                              {l.discount_percent} %
                            </TableCell>
                          )}
                          {d.calculation.tax_mode === "vat_payer" && (
                            <TableCell className="text-right whitespace-nowrap tabular-nums">
                              {l.vat_percent} %
                            </TableCell>
                          )}
                          <TableCell className="text-right whitespace-nowrap tabular-nums">
                            {moneyText(
                              totals.lines[i].amounts.gross,
                              language,
                              totals.currency,
                            )}
                          </TableCell>
                        </TableRow>
                      ))}
                    </tbody>
                  </Table>
                </div>
              </section>
              {invoice.advances && (
                <AdvanceDeductions
                  value={invoice.advances}
                  company={invoice.company_id}
                />
              )}
              <InvoiceSummary calculation={d.calculation} totals={totals} />
            </div>
          ),
        },
        {
          id: "attachments",
          label: t("invoiceAttachmentsTab"),
          content: (
            <DocumentAttachments
              company={invoice.company_id}
              kind="invoice"
              id={invoice.id}
              editable={canCopy}
            />
          ),
        },
        {
          id: "payments",
          label: t("paymentsTitle"),
          content: (
            <InvoicePayments
              currency={totals.currency}
              company={invoice.company_id}
              id={invoice.id}
              editable={canCopy}
              onSummary={setSettlement}
            />
          ),
        },
      ]}
    />
  );
  return (
    <div className="space-y-group">
      <Link
        className="text-sm text-muted hover:underline"
        to={`/companies/${invoice.company_id}/invoices`}
      >
        {t("navInvoices")}
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-field">
        <div>
          <div className="flex items-center gap-control">
            <Heading level={1} size="page">
              {t("invoiceDetailTitle", { number: invoice.number ?? "" })}
            </Heading>
          </div>
          <div className="mt-control">
            {settlement ? (
              <PaymentStatus value={settlement} />
            ) : (
              <Badge>{t("invoiceIssuedStatus")}</Badge>
            )}
          </div>
        </div>
        <div className="flex flex-wrap gap-control">
          {!historical && !invoice.advances && (
            <Button
              variant="secondary"
              loading={exporting}
              onClick={downloadIsdoc}
            >
              <Icon name="download" />
              {t("isdocDownload")}
            </Button>
          )}
          {canCopy && !historical && (
            <Link
              className="ui-button ui-button-secondary ui-button-md"
              to={`../new?copy=${encodeURIComponent(invoice.id)}`}
              relative="path"
            >
              <Icon name="copy" />
              {t("invoiceCopy")}
            </Link>
          )}
        </div>
      </div>
      {!historical && (
        <DocumentOutput company={invoice.company_id} id={invoice.id} />
      )}
      {exportError && (
        <Alert tone="danger" role="alert">
          {t("isdocError")}
        </Alert>
      )}
      <DocumentExports
        company={invoice.company_id}
        target={{ kind: "invoice", id: invoice.id, revision: invoice.revision }}
        editable={canCopy}
        owner={company?.role === "owner"}
      />
      <DocumentTags
        company={invoice.company_id}
        editable={canCopy}
        target={{ kind: "invoice", id: invoice.id }}
      />
      {historical && <Alert tone="warning">{t("importHistorical")}</Alert>}
      {historical ? (
        tabs
      ) : (
        <DocumentDelivery
          key={invoice.company_id + ":" + invoice.id}
          invoice={invoice}
          canSend={canCopy}
        >
          {tabs}
        </DocumentDelivery>
      )}
      {canCopy && !historical && (
        <Link
          className="ui-button ui-button-secondary ui-button-md"
          to={`/companies/${invoice.company_id}/recurring/new?invoice=${invoice.id}`}
        >
          {t("recFromInvoice")}
        </Link>
      )}
      {!historical && (
        <InvoiceCredits
          company={invoice.company_id}
          invoice={invoice.id}
          canCreate={canCopy}
        />
      )}
    </div>
  );
}
