import { useState } from "react";
import Decimal from "decimal.js";
import {
  Button,
  EmptyState,
  FilterTabs,
  Icon,
  IconButton,
  Input,
  Select,
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
} from "@open-faktura/ui";
import {
  displayDate,
  matchesFilter,
  money,
  remaining,
} from "../../../services/invoice-demo";
import type { DemoInvoice, InvoiceFilter } from "../../../types/invoice-demo";
import { InvoiceStatus } from "../../molecules/InvoiceStatus";

const filters: [InvoiceFilter, string][] = [
  ["all", "Všechny"],
  ["unpaid", "Neuhrazené"],
  ["overdue", "Po splatnosti"],
  ["paid", "Uhrazené"],
  ["draft", "Koncepty"],
];
export function InvoiceList({
  invoices,
  onOpen,
}: {
  invoices: DemoInvoice[];
  onOpen: (invoice: DemoInvoice) => void;
}) {
  const [filter, setFilter] = useState<InvoiceFilter>("all");
  const [search, setSearch] = useState("");
  const [period, setPeriod] = useState("all");
  const [sort, setSort] = useState("newest");
  const normalized = (s: string) =>
    s
      .normalize("NFD")
      .replace(/\p{Diacritic}/gu, "")
      .toLowerCase();
  const visible = invoices
    .filter(
      (i) =>
        matchesFilter(i, filter) &&
        normalized(`${i.id} ${i.customer} ${i.description}`).includes(
          normalized(search),
        ) &&
        (period === "all" || i.issuedOn.startsWith(period)),
    )
    .sort((a, b) =>
      sort === "amount"
        ? new Decimal(b.total).cmp(a.total)
        : b.issuedOn.localeCompare(a.issuedOn),
    );
  const summary = (f: InvoiceFilter, paid = false) =>
    invoices
      .filter((i) => !i.draft && matchesFilter(i, f))
      .reduce(
        (sum, i) => sum.plus(paid ? i.paid : remaining(i)),
        new Decimal(0),
      );
  return (
    <>
      <div className="mb-8 grid grid-cols-1 overflow-hidden rounded-xl border border-border bg-surface sm:grid-cols-3">
        {[
          {
            label: "Čeká na úhradu",
            value: summary("unpaid"),
            filter: "unpaid" as const,
            hint: "Včetně faktur po splatnosti",
            color: "text-foreground",
          },
          {
            label: "Po splatnosti",
            value: summary("overdue"),
            filter: "overdue" as const,
            hint: "Zbývající částka k úhradě",
            color: "text-danger",
          },
          {
            label: "Přijaté úhrady",
            value: summary("all", true),
            filter: "all" as const,
            hint: "Včetně částečných plateb",
            color: "text-success",
          },
        ].map((s, index) => (
          <button
            key={s.label}
            onClick={() => setFilter(s.filter)}
            className={`group p-5 text-left hover:bg-canvas/50 lg:p-6 ${index ? "border-t border-border sm:border-t-0 sm:border-l" : ""}`}
          >
            <span className="flex items-center justify-between text-xs font-medium text-muted">
              {s.label}
              <Icon
                name="arrow"
                className="h-4 w-4 opacity-0 transition-opacity group-hover:opacity-100"
              />
            </span>
            <span
              className={`mt-3 block text-2xl font-medium tracking-tight tabular-nums lg:text-3xl ${s.color}`}
            >
              {money(s.value)}
            </span>
            <span className="mt-2 block text-xs text-muted">{s.hint}</span>
          </button>
        ))}
      </div>
      <section
        aria-label="Seznam faktur"
        className="overflow-hidden rounded-xl border border-border bg-surface"
      >
        <FilterTabs
          label="Stav faktur"
          value={filter}
          onChange={setFilter}
          options={filters.map(([value, label]) => ({
            value,
            label,
            count: invoices.filter((i) => matchesFilter(i, value)).length,
          }))}
        />
        <div className="flex flex-col gap-control p-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="relative min-w-0 sm:w-72">
            <Icon
              name="search"
              className="pointer-events-none absolute top-3.5 left-3 h-4 w-4 text-muted"
            />
            <Input
              aria-label="Hledat faktury"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Hledat číslo, zákazníka, popis…"
              className="pl-9"
            />
          </div>
          <div className="flex min-w-0 flex-wrap gap-inline">
            <Select
              aria-label="Období vystavení"
              value={period}
              onChange={setPeriod}
              className="flex-1"
              options={[
                { value: "all", label: "Všechna období" },
                { value: "2026-10", label: "Říjen 2026" },
                { value: "2026-09", label: "Září 2026" },
              ]}
            />
            <Select
              aria-label="Řazení faktur"
              value={sort}
              onChange={setSort}
              className="flex-1"
              options={[
                { value: "newest", label: "Nejnovější první" },
                { value: "amount", label: "Nejvyšší částka" },
              ]}
            />
          </div>
        </div>
        <div className="hidden overflow-x-auto md:block">
          <Table aria-label="Vydané faktury">
            <TableHeader>
              <TableRow>
                <TableHead className="px-5 py-3 font-medium">
                  Doklad / zákazník
                </TableHead>
                <TableHead className="px-4 py-3 font-medium">
                  Stav úhrady
                </TableHead>
                <TableHead className="px-4 py-3 font-medium">
                  Splatnost
                </TableHead>
                <TableHead className="px-5 py-3 text-right font-medium">
                  Celkem / zbývá
                </TableHead>
                <TableHead className="w-10">
                  <span className="sr-only">Otevřít</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <tbody>
              {visible.map((i) => (
                <TableRow
                  key={i.id}
                  className="group border-b border-border/60 last:border-b-0 hover:bg-canvas/50"
                >
                  <TableCell className="px-5 py-4">
                    <button onClick={() => onOpen(i)} className="text-left">
                      <span className="font-semibold text-primary underline-offset-4 hover:underline">
                        {i.draft ? "Rozpracovaný doklad" : i.id}
                      </span>
                      <span className="ml-3 text-foreground">{i.customer}</span>
                      <span className="mt-1.5 block text-xs text-muted">
                        {i.description}
                      </span>
                    </button>
                  </TableCell>
                  <TableCell className="px-4 py-4">
                    <InvoiceStatus invoice={i} />
                  </TableCell>
                  <TableCell className="whitespace-nowrap px-4 py-4 text-xs">
                    {displayDate(i.dueOn)}
                  </TableCell>
                  <TableCell className="whitespace-nowrap px-5 py-4 text-right tabular-nums">
                    <span className="font-semibold">{money(i.total)}</span>
                    <span className="mt-1.5 block text-xs text-muted">
                      {i.draft
                        ? "Zatím nevystaveno"
                        : `Zbývá ${money(remaining(i))}`}
                    </span>
                  </TableCell>
                  <TableCell className="pr-4">
                    <IconButton
                      icon="arrow"
                      variant="ghost"
                      label={`Otevřít ${i.id}`}
                      onClick={() => onOpen(i)}
                    />
                  </TableCell>
                </TableRow>
              ))}
            </tbody>
          </Table>
        </div>
        <div className="md:hidden">
          {visible.map((i) => (
            <button
              key={i.id}
              onClick={() => onOpen(i)}
              className="block w-full border-t border-border p-4 text-left hover:bg-canvas"
            >
              <span className="flex justify-between gap-control">
                <span className="font-semibold">{i.customer}</span>
                <span className="whitespace-nowrap text-sm font-semibold tabular-nums">
                  {money(i.total)}
                </span>
              </span>
              <span className="mt-1 mb-3 block text-xs text-muted">
                {i.draft ? "Koncept" : i.id} · Splatnost {displayDate(i.dueOn)}
              </span>
              <InvoiceStatus invoice={i} />
            </button>
          ))}
        </div>
        {!visible.length && (
          <EmptyState
            title="Tady zatím žádná faktura není."
            description="Zkuste změnit hledání nebo vybrané filtry."
            action={
              <Button
                variant="secondary"
                onClick={() => {
                  setSearch("");
                  setFilter("all");
                  setPeriod("all");
                }}
              >
                Zrušit filtrování
              </Button>
            }
          />
        )}
        <footer className="flex justify-between gap-control border-t border-border px-5 py-4 text-xs text-muted">
          <span>
            Zobrazeno {visible.length} z {invoices.length} dokladů
          </span>
          <span>Částky v CZK</span>
        </footer>
      </section>
    </>
  );
}
