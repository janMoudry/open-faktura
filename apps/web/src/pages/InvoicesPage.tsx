import { Heading } from "@open-faktura/ui";
import { useEffect, useState } from "react";
import { Alert, Button, Icon } from "@open-faktura/ui";
import { WorkspaceLayout } from "../components/templates/WorkspaceLayout";
import { InvoiceList } from "../components/organisms/invoices/InvoiceList";
import { InvoiceEditor } from "../components/organisms/invoices/InvoiceEditor";
import { InvoiceStatus } from "../components/molecules/InvoiceStatus";
import {
  demoInvoices,
  displayDate,
  draftTotal,
  money,
  newDemoDraft,
  remaining,
} from "../services/invoice-demo";
import type { DemoInvoice } from "../types/invoice-demo";
import { DesignSystemPage } from "./DesignSystemPage";
import { HomePage } from "./HomePage";

export function InvoicesPage() {
  const [route, setRoute] = useState(location.hash || "#/invoices");
  const [invoices, setInvoices] = useState(demoInvoices);
  const [draft, setDraft] = useState(newDemoDraft);
  const [notice, setNotice] = useState("");
  useEffect(() => {
    const onHash = () => {
      setRoute(location.hash);
      window.scrollTo(0, 0);
    };
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);
  const editor = route === "#/invoices/new";
  const selected = route.startsWith("#/invoices/view/")
    ? invoices.find(
        (i) =>
          i.id === decodeURIComponent(route.slice("#/invoices/view/".length)),
      )
    : undefined;
  function open(invoice: DemoInvoice) {
    setNotice("");
    if (invoice.draft) {
      setDraft(invoice.draft);
      location.hash = "/invoices/new";
    } else {
      location.hash = `/invoices/view/${encodeURIComponent(invoice.id)}`;
    }
  }
  function save() {
    const invoice: DemoInvoice = {
      id: draft.id,
      customer: draft.customer,
      description: draft.lines[0].title,
      issuedOn: draft.issuedOn,
      dueOn: draft.dueOn,
      total: draftTotal(draft)!.toFixed(2),
      paid: "0",
      draft,
    };
    setInvoices((old) => [invoice, ...old.filter((i) => i.id !== invoice.id)]);
    setNotice("Ukázkový koncept byl uložen do této otevřené aplikace.");
    location.hash = "/invoices";
  }
  if (route === "#/design-system") return <DesignSystemPage />;
  if (route === "#/system")
    return (
      <>
        <a
          className="block bg-primary px-8 py-3 text-sm text-surface"
          href="#/invoices"
        >
          ← Zpět na faktury
        </a>
        <HomePage />
      </>
    );
  return (
    <WorkspaceLayout>
      {(editor || selected) && (
        <a
          href="#/invoices"
          className="mb-5 inline-flex items-center gap-inline text-xs text-muted hover:text-primary"
        >
          <Icon name="back" />
          Zpět na vydané faktury
        </a>
      )}
      <div className="mb-7 flex flex-wrap items-end justify-between gap-field">
        <div>
          <div className="mb-2 text-xs font-medium uppercase tracking-widest text-muted">
            Prodej /{" "}
            {editor ? "Nový doklad" : selected ? "Detail dokladu" : "Přehled"}
          </div>
          <Heading level={1} size="page">
            {editor
              ? "Nová faktura"
              : selected
                ? `Faktura ${selected.id}`
                : "Vydané faktury"}
            <span className="text-primary">.</span>
          </Heading>
          <p className="mt-3 text-sm text-muted">
            {editor
              ? "Zákazník, údaje dokladu a fakturované položky."
              : selected
                ? selected.customer
                : "Přehled dokladů, splatností a úhrad."}
          </p>
        </div>
        {!editor && !selected && (
          <Button
            onClick={() => {
              setDraft(newDemoDraft());
              setNotice("");
              location.hash = "/invoices/new";
            }}
          >
            <Icon name="plus" />
            Nová faktura
          </Button>
        )}
      </div>
      <div className="mb-6 flex items-center gap-control rounded-lg border border-border bg-surface/50 px-4 py-3 text-xs text-muted">
        <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-warning" />
        <p>
          Ukázková data k 10. 10. 2026.{" "}
          {editor
            ? "Vyzkoušejte si editor — doklad se nikam neodesílá."
            : "Interaktivní návrh vzhledu, bez zápisu do účetnictví."}
        </p>
      </div>
      {notice && (
        <Alert tone="success" role="status" className="mb-5">
          {notice}
        </Alert>
      )}
      {editor ? (
        <InvoiceEditor
          key={draft.id}
          draft={draft}
          onChange={setDraft}
          onSave={save}
        />
      ) : selected ? (
        <section className="max-w-2xl rounded-xl border border-border bg-surface p-7">
          <InvoiceStatus invoice={selected} />
          <p className="mt-6 font-display text-2xl">{selected.description}</p>
          <dl className="mt-6 grid grid-cols-2 gap-group text-sm">
            <div>
              <dt className="text-muted">Vystaveno</dt>
              <dd className="mt-2">{displayDate(selected.issuedOn)}</dd>
            </div>
            <div>
              <dt className="text-muted">Splatnost</dt>
              <dd className="mt-2">{displayDate(selected.dueOn)}</dd>
            </div>
            <div>
              <dt className="text-muted">Celkem</dt>
              <dd className="mt-2 text-xl">{money(selected.total)}</dd>
            </div>
            <div>
              <dt className="text-muted">Zbývá uhradit</dt>
              <dd className="mt-2 text-xl text-primary">
                {money(remaining(selected))}
              </dd>
            </div>
          </dl>
          <p className="mt-8 border-t border-border pt-5 text-xs text-muted">
            Ukázkový detail. Skutečné platby, PDF a odesílání doplníme s
            fakturačním modulem.
          </p>
        </section>
      ) : (
        <InvoiceList invoices={invoices} onOpen={open} />
      )}
      <p className="mt-6 text-xs text-muted">
        Open Faktura <span className="mx-2">/</span> Prostor pro vaši práci.
      </p>
    </WorkspaceLayout>
  );
}
