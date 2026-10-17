import { Link } from "react-router";
import type { AdvanceSettlement } from "@open-faktura/api-client";
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
export function AdvanceDeductions({
  value,
  company,
}: {
  value: AdvanceSettlement;
  company: string;
}) {
  const { t, language } = useAppTranslation();
  return (
    <section className="invoice-section space-y-field">
      <Heading level={2} size="section">
        {t("settlementDeductions")}
      </Heading>
      <p className="text-sm text-muted">{t("settlementStoredTax")}</p>
      {value.sources.map((s) => (
        <div key={s.id} className="space-y-control">
          <div className="flex flex-wrap justify-between gap-field">
            <Link
              className="underline"
              to={`/companies/${company}/advances/${s.id}`}
            >
              {s.number}
            </Link>
            <span className="whitespace-nowrap">
              −{moneyText(s.paid, language)}
            </span>
          </div>
          {s.tax_receipts.map((r) => (
            <div key={r.id}>
              <Link
                className="text-sm underline"
                to={`/companies/${company}/tax-receipts/${r.id}`}
              >
                {r.number}
              </Link>
              <div className="overflow-x-auto">
                <Table className="whitespace-nowrap">
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("taxRate")}</TableHead>
                      <TableHead>{t("taxNet")}</TableHead>
                      <TableHead>{t("taxVat")}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <tbody>
                    {r.totals.tax_summary.map((g) => (
                      <TableRow key={g.vat_percent}>
                        <TableCell>{g.vat_percent} %</TableCell>
                        <TableCell>
                          −{moneyText(g.amounts.net, language)}
                        </TableCell>
                        <TableCell>
                          −{moneyText(g.amounts.vat, language)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </tbody>
                </Table>
              </div>
            </div>
          ))}
        </div>
      ))}
      <p className="font-semibold text-right">
        {t("settlementCredited")}: −{moneyText(value.credited, language)}
      </p>
    </section>
  );
}
