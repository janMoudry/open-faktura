import Decimal from "decimal.js";
import type {
  DemoDraft,
  DemoInvoice,
  DemoLine,
  InvoiceFilter,
} from "../types/invoice-demo";

// Isolated design fixtures. Not an API adapter or the production accounting calculator.
export const demoToday = "2026-10-10";
export const demoInvoices: DemoInvoice[] = [
  {
    id: "2026-042",
    customer: "Studio Sever",
    description: "Vizuální identita · říjen",
    issuedOn: "2026-10-09",
    dueOn: "2026-10-23",
    total: "36000",
    paid: "0",
  },
  {
    id: "2026-041",
    customer: "Kavárna Mezi řádky",
    description: "Webové stránky",
    issuedOn: "2026-10-06",
    dueOn: "2026-10-20",
    total: "18500",
    paid: "18500",
  },
  {
    id: "2026-040",
    customer: "Ateliér Forma",
    description: "Návrh katalogu",
    issuedOn: "2026-10-01",
    dueOn: "2026-10-08",
    total: "24000",
    paid: "10000",
  },
  {
    id: "2026-039",
    customer: "Dobré místo",
    description: "Konzultace a strategie",
    issuedOn: "2026-10-01",
    dueOn: "2026-10-15",
    total: "8500",
    paid: "0",
  },
  {
    id: "2026-038",
    customer: "Papír & příběh",
    description: "Ilustrace pro nový obal",
    issuedOn: "2026-09-25",
    dueOn: "2026-10-09",
    total: "12800",
    paid: "0",
  },
  {
    id: "2026-037",
    customer: "Zahrada ve městě",
    description: "Správa webu · září",
    issuedOn: "2026-09-22",
    dueOn: "2026-10-06",
    total: "6200",
    paid: "6200",
  },
];
export function newDemoDraft(): DemoDraft {
  return {
    id: crypto.randomUUID(),
    customer: "",
    address: "",
    issuedOn: demoToday,
    dueOn: "2026-10-24",
    note: "Děkujeme za spolupráci.",
    lines: [
      {
        id: crypto.randomUUID(),
        title: "Návrh vizuální identity",
        quantity: "1",
        unit: "ks",
        price: "24000",
      },
      {
        id: crypto.randomUUID(),
        title: "Konzultace a příprava podkladů",
        quantity: "4",
        unit: "hod",
        price: "1500",
      },
    ],
  };
}
const decimalInput = /^\d{1,9}([.,]\d{1,2})?$/;
export function lineTotal(line: DemoLine): Decimal | null {
  if (!decimalInput.test(line.quantity) || !decimalInput.test(line.price))
    return null;
  const quantity = new Decimal(line.quantity.replace(",", "."));
  if (quantity.lte(0)) return null;
  return quantity
    .mul(line.price.replace(",", "."))
    .toDecimalPlaces(2, Decimal.ROUND_HALF_UP);
}
export function draftTotal(draft: DemoDraft): Decimal | null {
  let total = new Decimal(0);
  for (const line of draft.lines) {
    const value = lineTotal(line);
    if (value === null) return null;
    total = total.plus(value);
  }
  return draft.lines.length ? total : null;
}
export function money(value: Decimal.Value) {
  const [whole, cents] = new Decimal(value).toFixed(2).split(".");
  return `${whole.replace(/\B(?=(\d{3})+(?!\d))/g, " ")},${cents} Kč`;
}
export function displayDate(value: string) {
  return value.split("-").reverse().join(". ");
}
export function remaining(invoice: DemoInvoice) {
  return new Decimal(invoice.total).minus(invoice.paid);
}
export function matchesFilter(invoice: DemoInvoice, filter: InvoiceFilter) {
  if (filter === "all") return true;
  if (filter === "draft") return Boolean(invoice.draft);
  if (invoice.draft) return false;
  if (filter === "paid") return remaining(invoice).eq(0);
  return (
    remaining(invoice).gt(0) &&
    (filter !== "overdue" || invoice.dueOn < demoToday)
  );
}
