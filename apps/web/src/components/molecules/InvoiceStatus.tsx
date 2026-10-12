import { Badge } from "@open-faktura/ui";
import { demoToday, remaining } from "../../services/invoice-demo";
import type { DemoInvoice } from "../../types/invoice-demo";
export function InvoiceStatus({ invoice }: { invoice: DemoInvoice }) {
  const paid = remaining(invoice).eq(0);
  const overdue = !invoice.draft && !paid && invoice.dueOn < demoToday;
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <Badge tone={invoice.draft ? "neutral" : paid ? "success" : "warning"}>
        {invoice.draft
          ? "Koncept"
          : paid
            ? "Uhrazeno"
            : invoice.paid !== "0"
              ? "Částečně uhrazeno"
              : "Neuhrazeno"}
      </Badge>
      {overdue && <Badge tone="danger">Po splatnosti</Badge>}
    </div>
  );
}
