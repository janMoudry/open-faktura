export interface DemoLine {
  id: string;
  title: string;
  quantity: string;
  price: string;
  unit: string;
}
export interface DemoDraft {
  id: string;
  customer: string;
  address: string;
  issuedOn: string;
  dueOn: string;
  note: string;
  lines: DemoLine[];
}
export interface DemoInvoice {
  id: string;
  customer: string;
  description: string;
  issuedOn: string;
  dueOn: string;
  total: string;
  paid: string;
  draft?: DemoDraft;
}
export type InvoiceFilter = "all" | "unpaid" | "overdue" | "paid" | "draft";
