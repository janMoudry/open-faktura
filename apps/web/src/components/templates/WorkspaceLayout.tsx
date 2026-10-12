import type { ReactNode } from "react";
import { Icon } from "@open-faktura/ui";
export function WorkspaceLayout({
  children,
  active = "invoices",
}: {
  children: ReactNode;
  active?: "invoices" | "design-system";
}) {
  return (
    <div className="min-h-screen lg:flex">
      <a
        href="#main-content"
        onClick={(e) => {
          e.preventDefault();
          document.getElementById("main-content")?.focus();
        }}
        className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:rounded-lg focus:bg-surface focus:p-3"
      >
        Přejít na obsah
      </a>
      <aside className="flex shrink-0 flex-col border-b border-border bg-sidebar lg:fixed lg:inset-y-0 lg:w-60 lg:overflow-y-auto lg:border-r lg:border-b-0">
        <a
          href="#/invoices"
          className="flex items-center gap-inline whitespace-nowrap px-5 py-7 text-lg font-semibold tracking-tight"
        >
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary font-display text-2xl text-surface">
            f.
          </span>
          open faktura
          <Icon name="external" className="h-4 w-4 text-primary" />
        </a>
        <div className="mx-4 hidden rounded-xl border border-border bg-surface/60 p-3 lg:flex lg:items-center lg:gap-control">
          <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-canvas text-sm text-muted">
            S
          </span>
          <div>
            <p className="text-sm font-semibold">Studio Nový den</p>
            <p className="mt-0.5 text-xs text-muted">Ukázková firma · CZK</p>
          </div>
        </div>
        <nav
          aria-label="Hlavní navigace"
          className="flex flex-wrap gap-inline px-4 pb-3 lg:mt-8 lg:flex-col"
        >
          <p className="mb-2 hidden px-3 text-xs font-medium uppercase tracking-widest text-muted lg:block">
            Pracovní prostor
          </p>
          <a
            href="#/invoices"
            aria-current={active === "invoices" ? "page" : undefined}
            className={`flex items-center gap-control rounded-lg px-3 py-3 text-sm ${active === "invoices" ? "bg-primary-soft font-semibold text-primary" : "text-muted hover:bg-surface"}`}
          >
            <Icon name="invoice" />
            Vydané faktury
          </a>
          <a
            href="#/system"
            className="flex items-center gap-control rounded-lg px-3 py-3 text-sm text-muted hover:bg-surface"
          >
            <Icon name="settings" />
            Stav systému
          </a>
          <a
            href="#/design-system"
            aria-current={active === "design-system" ? "page" : undefined}
            className={`flex items-center gap-control rounded-lg px-3 py-3 text-sm ${active === "design-system" ? "bg-primary-soft font-semibold text-primary" : "text-muted hover:bg-surface"}`}
          >
            <Icon name="settings" />
            Komponenty
          </a>
        </nav>
        <div className="mt-auto hidden p-6 lg:block">
          <div className="mb-6 border-t border-border pt-5 text-xs leading-relaxed text-muted">
            Otevřený software.
            <br />
            Vaše data, vaše fakturace.
          </div>
          <div className="flex items-center gap-control">
            <span className="flex h-8 w-8 items-center justify-center rounded-full bg-primary text-xs text-surface">
              ND
            </span>
            <div className="text-xs">
              <p className="font-semibold">Návrhové prostředí</p>
              <p className="mt-1 text-muted">Open Faktura · 0.1</p>
            </div>
          </div>
        </div>
      </aside>
      <div className="min-w-0 flex-1 lg:ml-60">
        <header className="flex items-center justify-between gap-field border-b border-border bg-surface/60 px-5 py-4 text-xs text-muted lg:px-10">
          <span>
            Pracovní prostor <span className="mx-2 text-border">/</span>{" "}
            {active === "design-system" ? "Design systém" : "Prodej"}
          </span>
          <span className="rounded-full border border-border px-2.5 py-1 text-primary">
            Design preview
          </span>
        </header>
        <main
          id="main-content"
          tabIndex={-1}
          className="mx-auto max-w-screen-2xl px-5 py-7 lg:px-10 lg:py-9"
        >
          {children}
        </main>
      </div>
    </div>
  );
}
