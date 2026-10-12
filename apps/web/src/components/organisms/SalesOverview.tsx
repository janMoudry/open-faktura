import type { SalesDocument } from "@open-faktura/api-client";
import {
  Heading,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
import { moneyText } from "../../utils/invoice";
import { salesStates } from "../../utils/sales";
import { InvoiceSummary } from "./InvoiceSummary";
/** Same document sections as the editor, with readable facts instead of disabled inputs. */
export function SalesOverview({ document: d }: { document: SalesDocument }) {
  const { t, language } = useAppTranslation();
  const v = d.data.invoice;
  const date = (v: string | null) =>
    v
      ? new Intl.DateTimeFormat(language).format(new Date(v + "T12:00:00Z"))
      : "—";
  const fact = (label: string, value: string) => (
    <div key={label}>
      <dt>{label}</dt>
      <dd>{value || "—"}</dd>
    </div>
  );
  return (
    <article className="invoice-sheet">
      <section className="invoice-section">
        <dl className="invoice-facts max-w-3xl mx-auto">
          {fact(
            t(d.kind === "quote" ? "salesDescription" : "salesName"),
            d.data.name,
          )}
          {fact(t("draftCustomer"), v.customer?.name ?? "")}
          {v.customer &&
            fact(
              t("salesCustomerDetails"),
              [
                v.customer.address.street,
                `${v.customer.address.postal_code} ${v.customer.address.city}`.trim(),
                v.customer.registration_number,
              ]
                .filter(Boolean)
                .join(", "),
            )}
          {d.kind === "quote" &&
            fact(
              t("salesStatus"),
              t(salesStates[d.data.status] ?? "salesCreated"),
            )}
          {fact(t("draftOrder"), v.order_number)}
          {d.kind === "template" && fact(t("invoiceDescription"), v.note)}
        </dl>
      </section>
      <section className="invoice-section invoice-form-facts bg-sidebar grid gap-section md:grid-cols-2">
        <div>
          <Heading level={2} size="card">
            {t("salesDates")}
          </Heading>
          <dl className="invoice-facts mt-field">
            {d.kind === "quote" && (
              <>
                {fact(t("draftIssued"), date(v.issued_on))}
                {fact(t("salesValidUntil"), date(v.due_on))}
              </>
            )}
            {fact(t("salesMaturity"), String(d.data.maturity_days))}
          </dl>
        </div>
        <div>
          <Heading level={2} size="card">
            {t("salesPayment")}
          </Heading>
          <dl className="invoice-facts mt-field">
            {fact(
              t("paymentMethod"),
              t(v.payment.method === "cash" ? "cash" : "bankTransfer"),
            )}
            {fact(t("contactBankAccount"), v.payment.bank_account)}
            {fact(t("iban"), v.payment.iban)}
            {fact(
              t("draftLanguage"),
              v.language === "cs" ? "Čeština" : "English",
            )}
          </dl>
        </div>
      </section>
      <section className="invoice-section">
        <details>
          <summary className="text-sm cursor-pointer">
            {t("draftSupplier")}
          </summary>
          <p className="mt-field">
            {v.supplier.name}
            <br />
            {v.supplier.address.street}
            <br />
            {v.supplier.address.postal_code} {v.supplier.address.city}
          </p>
        </details>
      </section>
      <section className="invoice-section">
        <Heading level={2} size="card">
          {t("salesItems")}
        </Heading>
        <div
          className="overflow-x-auto mt-field"
          tabIndex={0}
          role="region"
          aria-label={t("salesItems")}
        >
          <Table className="contact-register-table min-w-invoice-register">
            <TableHeader>
              <TableRow>
                <TableHead>{t("draftDescription")}</TableHead>
                <TableHead>{t("draftUnitPrice")}</TableHead>
                <TableHead>{t("draftQuantity")}</TableHead>
                <TableHead>{t("draftUnit")}</TableHead>
                <TableHead className="text-right">
                  {t("invoiceColumnAmount")}
                </TableHead>
              </TableRow>
            </TableHeader>
            <tbody>
              {v.calculation.lines.map((line) => (
                <TableRow key={line.id}>
                  <TableCell>{line.description || "—"}</TableCell>
                  <TableCell>
                    {line.unit_price
                      ? moneyText(line.unit_price, language)
                      : "—"}
                  </TableCell>
                  <TableCell>{line.quantity}</TableCell>
                  <TableCell>{line.unit || "—"}</TableCell>
                  <TableCell className="text-right tabular-nums">
                    {d.totals
                      ? moneyText(
                          d.totals.lines.find((l) => l.id === line.id)?.amounts
                            .gross ?? "0",
                          language,
                        )
                      : "—"}
                  </TableCell>
                </TableRow>
              ))}
            </tbody>
          </Table>
        </div>
      </section>
      {d.totals && (
        <InvoiceSummary
          totalLabel={d.kind === "quote" ? t("quoteTotalPrice") : undefined}
          calculation={v.calculation}
          totals={d.totals}
        />
      )}
    </article>
  );
}
