import { Link, useLocation } from "react-router";
import { useAppTranslation, type AppKey } from "../../locales";

const pages: Record<string, AppKey> = {
  overview: "navOverview",
  invoices: "navInvoices",
  quotes: "navQuotes",
  advances: "navAdvances",
  recurring: "navRecurring",
  "credit-notes": "navCreditNotes",
  "tax-receipts": "taxReceipts",
  templates: "salesTemplates",
  contacts: "navContacts",
  catalog: "catalogList",
  stock: "catalogStock",
  bank: "navBank",
  cash: "navCash",
  reports: "navReports",
  "received-invoices": "receivedTitle",
  receipts: "receiptsTitle",
  inbox: "inboxTitle",
  imports: "navImport",
  retail: "retailTitle",
  "expense-payments": "expensePayments",
  "expense-overview": "expenseOverview",
};

export function WorkspaceBreadcrumb() {
  const { pathname } = useLocation();
  const { t } = useAppTranslation();
  const parts = pathname.split("/").filter(Boolean);
  const page = pages[parts[2]];
  return (
    <nav aria-label={t("breadcrumbLabel")} className="workspace-breadcrumb">
      <Link to="/workspace">{t("brandName")}</Link>
      <span aria-hidden="true">/</span>
      <span aria-current="page">{t(page ?? "companySettingsIntro")}</span>
    </nav>
  );
}
