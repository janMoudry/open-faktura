import {
  memo,
  useEffect,
  useState,
  type ReactNode,
  type ComponentProps,
} from "react";
import {
  Link,
  NavLink,
  useLocation,
  type NavLinkRenderProps,
} from "react-router";
import { Button, Icon } from "@open-faktura/ui";
import { useAppTranslation, type AppKey } from "../../locales";
type IconName = ComponentProps<typeof Icon>["name"];
export const ProductSidebar = memo(function ProductSidebar({
  selected,
  canCreate = false,
}: {
  selected?: string;
  canCreate?: boolean;
}) {
  const { t } = useAppTranslation();
  const destinations: Partial<Record<AppKey, string>> = selected
    ? {
        receiptsTitle: `/companies/${selected}/receipts`,
        expensePayments: `/companies/${selected}/expense-payments`,
        expenseOverview: `/companies/${selected}/expense-overview`,
        navReports: `/companies/${selected}/reports`,
        catalogStock: `/companies/${selected}/stock`,
        inboxTitle: `/companies/${selected}/inbox`,
        retailTitle: `/companies/${selected}/retail`,
        navCash: `/companies/${selected}/cash`,
        navBank: `/companies/${selected}/bank`,
        receivedTitle: `/companies/${selected}/received-invoices`,
        navRecurring: `/companies/${selected}/recurring`,
        navCreditNotes: `/companies/${selected}/credit-notes`,
        taxReceipts: `/companies/${selected}/tax-receipts`,
        navAdvances: `/companies/${selected}/advances`,
        navQuotes: `/companies/${selected}/quotes`,
        salesTemplates: `/companies/${selected}/templates`,
        navInvoices: `/companies/${selected}/invoices`,
        catalogList: `/companies/${selected}/catalog`,
        navContacts: `/companies/${selected}/contacts`,
        navImport: `/companies/${selected}/imports`,
        companySettingsIntro: `/companies/${selected}`,
      }
    : {};
  const unavailable = (label: AppKey, icon?: IconName) => (
    <button
      key={label}
      type="button"
      disabled
      title={t("navSoon")}
      aria-label={`${t(label)} — ${t("navSoon")}`}
      className="workspace-nav-item disabled:cursor-not-allowed text-muted"
    >
      {icon && <Icon name={icon} />}
      <span>{t(label)}</span>
    </button>
  );
  const group = (label: AppKey, icon: IconName, items: AppKey[]) => (
    <NavigationGroup
      paths={items.flatMap((item) =>
        destinations[item] ? [destinations[item]!] : [],
      )}
    >
      <summary className="workspace-nav-item">
        <Icon name={icon} />
        <span className="flex-1">{t(label)}</span>
        <Icon name="chevronDown" />
      </summary>
      <div className="workspace-nav-children">
        {items.map((item) =>
          destinations[item] ? (
            <NavLink
              key={item}
              to={destinations[item]!}
              end={item === "companySettingsIntro"}
              className={({ isActive }: NavLinkRenderProps) =>
                `workspace-nav-item ${isActive ? "font-semibold" : "text-muted"}`
              }
            >
              {t(item)}
            </NavLink>
          ) : (
            unavailable(item)
          ),
        )}
      </div>
    </NavigationGroup>
  );
  const navigation = (
    <nav aria-label={t("navMain")} className="space-y-inline">
      {selected ? (
        <NavLink
          to={`/companies/${selected}/overview`}
          className={({ isActive }: NavLinkRenderProps) =>
            `workspace-nav-item ${isActive ? "font-semibold" : "text-muted"}`
          }
        >
          <Icon name="overview" />
          {t("navOverview")}
        </NavLink>
      ) : (
        unavailable("navOverview", "overview")
      )}
      {group("navSales", "invoice", [
        "navInvoices",
        "retailTitle",
        "navQuotes",
        "salesTemplates",
        "navAdvances",
        "taxReceipts",
        "navRecurring",
        "navCreditNotes",
      ])}
      {group("navExpenses", "expenses", [
        "inboxTitle",
        "receivedTitle",
        "receiptsTitle",
        "expensePayments",
        "expenseOverview",
      ])}
      {group("navDirectory", "contacts", ["navContacts", "navMailHistory"])}
      {group("navCatalog", "catalog", ["catalogList", "catalogStock"])}
      {group("navBusiness", "wallet", ["navBank", "navCash", "navReports"])}
      {group("navSettings", "settings", ["companySettingsIntro", "navTeam"])}
      {group("navTools", "tools", ["navImport", "navIntegrations"])}
    </nav>
  );
  return (
    <aside className="workspace-sidebar">
      <div className="workspace-create px-inline py-field">
        {selected && canCreate ? (
          <Link
            className="ui-button ui-button-primary ui-button-md w-full"
            to={`/companies/${selected}/invoices/new`}
          >
            <Icon name="plus" />
            {t("navNewInvoice")}
          </Link>
        ) : (
          <Button className="w-full" disabled>
            <Icon name="plus" />
            {t("navNewInvoice")}
          </Button>
        )}
      </div>
      <div className="hidden px-inline lg:block">{navigation}</div>
      <details className="px-field pb-field lg:hidden">
        <summary className="workspace-nav-item font-semibold">
          <Icon name="menu" />
          {t("navMain")}
        </summary>
        {navigation}
      </details>
      <p className="mt-auto hidden gap-inline p-group text-xs leading-relaxed text-muted lg:flex">
        <Icon name="info" />
        {t("navSoonHint")}
      </p>
    </aside>
  );
});

// Only route-aware leaves subscribe to location. The shell stays memoized.
function NavigationGroup({
  paths,
  children,
}: {
  paths: string[];
  children: ReactNode;
}) {
  const { pathname } = useLocation();
  const active = paths.some(
    (path) =>
      pathname === path ||
      (path.split("/").length > 3 && pathname.startsWith(path + "/")),
  );
  const [open, setOpen] = useState(active);
  useEffect(() => {
    if (active) setOpen(true);
  }, [active]);
  return (
    <details
      className="workspace-nav-group"
      open={open}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      {children}
    </details>
  );
}
