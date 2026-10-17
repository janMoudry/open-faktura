import type { TaxReceipt } from "@open-faktura/api-client";
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
} from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
import { moneyText } from "../../utils/invoice";
export function TaxReceiptSummary({ document: d }: { document: TaxReceipt }) {
  const { t, language } = useAppTranslation();
  return (
    <div className="overflow-x-auto">
      <Table className="whitespace-nowrap">
        <TableHeader>
          <TableRow>
            <TableHead>{t("taxRate")}</TableHead>
            <TableHead>{t("taxNet")}</TableHead>
            <TableHead>{t("taxVat")}</TableHead>
            <TableHead>{t("taxGross")}</TableHead>
          </TableRow>
        </TableHeader>
        <tbody>
          {d.totals.tax_summary.map((g) => (
            <TableRow key={g.vat_percent}>
              <TableCell>{g.vat_percent} %</TableCell>
              <TableCell>{moneyText(g.amounts.net, language)}</TableCell>
              <TableCell>{moneyText(g.amounts.vat, language)}</TableCell>
              <TableCell>{moneyText(g.amounts.gross, language)}</TableCell>
            </TableRow>
          ))}
        </tbody>
      </Table>
      <p className="p-field font-semibold">
        {t("taxReceived")}: {moneyText(d.totals.payable, language)}
      </p>
    </div>
  );
}
