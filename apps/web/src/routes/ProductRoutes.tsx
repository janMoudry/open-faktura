import { lazy, Suspense } from "react";
import {
  createHashRouter,
  Link,
  Navigate,
  Outlet,
  RouterProvider,
  useLocation,
  useNavigate,
} from "react-router";
import { Alert, Button, Heading } from "@open-faktura/ui";
import { AppLayout } from "../components/templates/AppLayout";
import { WorkspaceLayout } from "../components/templates/ProductRouteLayout";
import { useAuthSession } from "../hooks/useAuthSession";
import { AuthPage } from "../pages/AuthPage";
import { useAppTranslation } from "../locales";
import { NotFoundPage } from "../pages/NotFoundPage";

const ReportsPage = lazy(() =>
  import("../pages/ReportsPage").then((m) => ({ default: m.ReportsPage })),
);
const ItemSalesPage = lazy(() =>
  import("../pages/ItemSalesPage").then((m) => ({ default: m.ItemSalesPage })),
);
const StockPage = lazy(() =>
  import("../pages/StockPage").then((m) => ({ default: m.StockPage })),
);
const InboxPage = lazy(() =>
  import("../pages/InboxPage").then((m) => ({ default: m.InboxPage })),
);
const RetailPage = lazy(() =>
  import("../pages/RetailPage").then((m) => ({ default: m.RetailPage })),
);
const BankPage = lazy(() =>
  import("../pages/BankPage").then((m) => ({ default: m.BankPage })),
);
const BankMovementPage = lazy(() =>
  import("../pages/BankMovementPage").then((m) => ({
    default: m.BankMovementPage,
  })),
);
const CashPage = lazy(() =>
  import("../pages/CashPage").then((m) => ({ default: m.CashPage })),
);
const CashMovementPage = lazy(() =>
  import("../pages/CashMovementPage").then((m) => ({
    default: m.CashMovementPage,
  })),
);
const ReceivedInvoicesPage = lazy(() =>
  import("../pages/ReceivedInvoicesPage").then((m) => ({
    default: m.ReceivedInvoicesPage,
  })),
);
const ReceivedInvoicePage = lazy(() =>
  import("../pages/ReceivedInvoicePage").then((m) => ({
    default: m.ReceivedInvoicePage,
  })),
);
const PurchaseReportPage = lazy(() =>
  import("../pages/PurchaseReportPage").then((m) => ({
    default: m.PurchaseReportPage,
  })),
);
const RecurringListPage = lazy(() =>
  import("../pages/RecurringListPage").then((m) => ({
    default: m.RecurringListPage,
  })),
);
const RecurringDetailPage = lazy(() =>
  import("../pages/RecurringDetailPage").then((m) => ({
    default: m.RecurringDetailPage,
  })),
);
const CreditNotesPage = lazy(() =>
  import("../pages/CreditNotesPage").then((m) => ({
    default: m.CreditNotesPage,
  })),
);
const CreditNotePage = lazy(() =>
  import("../pages/CreditNotePage").then((m) => ({
    default: m.CreditNotePage,
  })),
);
const TaxReceiptsPage = lazy(() =>
  import("../pages/TaxReceiptsPage").then((m) => ({
    default: m.TaxReceiptsPage,
  })),
);
const TaxReceiptPage = lazy(() =>
  import("../pages/TaxReceiptPage").then((m) => ({
    default: m.TaxReceiptPage,
  })),
);
const PublicInvoicePage = lazy(() =>
  import("../pages/PublicInvoicePage").then((m) => ({
    default: m.PublicInvoicePage,
  })),
);
const PublicQuotePage = lazy(() =>
  import("../pages/PublicQuotePage").then((m) => ({
    default: m.PublicQuotePage,
  })),
);
const SalesListPage = lazy(() =>
  import("../pages/SalesListPage").then((m) => ({ default: m.SalesListPage })),
);
const SalesDetailPage = lazy(() =>
  import("../pages/SalesDetailPage").then((m) => ({
    default: m.SalesDetailPage,
  })),
);
const CatalogPage = lazy(() =>
  import("../pages/CatalogPage").then((m) => ({ default: m.CatalogPage })),
);
const CatalogDetailPage = lazy(() =>
  import("../pages/CatalogDetailPage").then((m) => ({
    default: m.CatalogDetailPage,
  })),
);
const ImportPage = lazy(() =>
  import("../pages/ImportPage").then((m) => ({ default: m.ImportPage })),
);
const OverviewPage = lazy(() =>
  import("../pages/OverviewPage").then((m) => ({ default: m.OverviewPage })),
);
const InvoiceDraftListPage = lazy(() =>
  import("../pages/InvoiceDraftListPage").then((m) => ({
    default: m.InvoiceDraftListPage,
  })),
);
const InvoiceDraftPage = lazy(() =>
  import("../pages/InvoiceDraftPage").then((m) => ({
    default: m.InvoiceDraftPage,
  })),
);
const CompanyPage = lazy(() =>
  import("../pages/CompanyPage").then((module) => ({
    default: module.CompanyPage,
  })),
);
const ContactsPage = lazy(() =>
  import("../pages/ContactsPage").then((module) => ({
    default: module.ContactsPage,
  })),
);
const ContactDetailPage = lazy(() =>
  import("../pages/ContactDetailPage").then((module) => ({
    default: module.ContactDetailPage,
  })),
);
const HomePage = lazy(() =>
  import("../pages/HomePage").then((module) => ({ default: module.HomePage })),
);
export type SessionContext = {
  user: import("@open-faktura/api-client").AuthUser;
  setUser: (user: import("@open-faktura/api-client").AuthUser | null) => void;
  expire: () => void;
};
function AccessLayout() {
  const { t } = useAppTranslation();
  const { access, refresh, setUser, expire } = useAuthSession();
  const { pathname } = useLocation();
  const navigate = useNavigate();
  if (access.kind === "ready" && access.user)
    return (
      <Outlet
        context={
          { user: access.user, setUser, expire } satisfies SessionContext
        }
      />
    );
  const known =
    /^\/(?:login|setup|workspace|companies(?:\/[^/]+(?:\/(?:reports|stock|inbox|retail|cash|bank|contacts|invoices|received-invoices|receipts|expense-payments|expense-overview|overview|imports|catalog|quotes|templates|advances|tax-receipts|credit-notes|recurring)(?:\/[^/]+)?)?)?|invoices(?:\/.*)?)?$/.test(
      pathname,
    );
  return (
    <AppLayout>
      <div className="mx-auto max-w-lg">
        {!known ? (
          <NotFoundPage />
        ) : access.kind === "loading" ? (
          <p role="status">{t("checkingAccess")}</p>
        ) : access.kind === "error" ? (
          <>
            <Heading level={1} size="page">
              {t("unavailable")}
            </Heading>
            <Alert tone="warning" role="alert" className="mt-group">
              {t("setupFailed")}
            </Alert>
            <Button className="mt-group" onClick={refresh}>
              {t("retry")}
            </Button>
          </>
        ) : (
          <AuthPage
            key={`${pathname}-${access.setup}`}
            setup={access.setup && pathname !== "/login"}
            onDone={() => {
              refresh();
              void navigate("/workspace");
            }}
          />
        )}
        <Link
          to="/system"
          className="mt-section block text-xs text-muted underline underline-offset-4"
        >
          {t("connectionDetails")}
        </Link>
      </div>
    </AppLayout>
  );
}
function UnavailablePage() {
  const { t } = useAppTranslation();
  return (
    <>
      <Heading level={1} size="page">
        {t("navInvoices")}
      </Heading>
      <p className="mt-field text-muted">{t("navSoon")}</p>
    </>
  );
}
const router = createHashRouter([
  {
    path: "/shared/invoice/:token",
    element: (
      <Suspense fallback={null}>
        <PublicInvoicePage />
      </Suspense>
    ),
  },
  {
    path: "/shared/quote/:token",
    element: (
      <Suspense fallback={null}>
        <PublicQuotePage />
      </Suspense>
    ),
  },
  {
    path: "/system",
    element: (
      <Suspense fallback={null}>
        <HomePage />
      </Suspense>
    ),
  },
  {
    element: <AccessLayout />,
    children: [
      {
        element: <WorkspaceLayout />,
        children: [
          { index: true, element: <CompanyPage /> },
          { path: "workspace", element: <CompanyPage /> },
          { path: "companies", element: <CompanyPage /> },
          { path: "companies/:companyId", element: <CompanyPage /> },
          { path: "companies/:companyId/bank", element: <BankPage /> },
          {
            path: "companies/:companyId/bank/:movementId",
            element: <BankMovementPage />,
          },
          { path: "companies/:companyId/cash", element: <CashPage /> },
          {
            path: "companies/:companyId/cash/:movementId",
            element: <CashMovementPage />,
          },
          {
            path: "companies/:companyId/received-invoices",
            element: <ReceivedInvoicesPage />,
          },
          {
            path: "companies/:companyId/received-invoices/:receivedId",
            element: <ReceivedInvoicePage />,
          },
          {
            path: "companies/:companyId/receipts",
            element: <ReceivedInvoicesPage kind="receipt" />,
          },
          {
            path: "companies/:companyId/receipts/:receivedId",
            element: <ReceivedInvoicePage kind="receipt" />,
          },
          {
            path: "companies/:companyId/expense-payments",
            element: <PurchaseReportPage key="payments" view="payments" />,
          },
          {
            path: "companies/:companyId/expense-overview",
            element: <PurchaseReportPage key="overview" view="overview" />,
          },
          { path: "companies/:companyId/overview", element: <OverviewPage /> },
          { path: "companies/:companyId/imports", element: <ImportPage /> },
          {
            path: "companies/:companyId/recurring",
            element: <RecurringListPage />,
          },
          {
            path: "companies/:companyId/recurring/:recurringId",
            element: <RecurringDetailPage />,
          },
          {
            path: "companies/:companyId/reports",
            element: <ReportsPage />,
          },
          {
            path: "companies/:companyId/item-sales",
            element: <ItemSalesPage />,
          },
          {
            path: "companies/:companyId/stock",
            element: <StockPage />,
          },
          {
            path: "companies/:companyId/inbox",
            element: <InboxPage />,
          },
          {
            path: "companies/:companyId/retail",
            element: <RetailPage />,
          },
          {
            path: "companies/:companyId/retail/:retailId",
            element: <RetailPage />,
          },
          {
            path: "companies/:companyId/credit-notes",
            element: <CreditNotesPage />,
          },
          {
            path: "companies/:companyId/credit-notes/:creditId",
            element: <CreditNotePage />,
          },
          {
            path: "companies/:companyId/tax-receipts",
            element: <TaxReceiptsPage />,
          },
          {
            path: "companies/:companyId/tax-receipts/:taxId",
            element: <TaxReceiptPage />,
          },
          {
            path: "companies/:companyId/advances",
            element: <SalesListPage kind="advance" />,
          },
          {
            path: "companies/:companyId/advances/:salesId",
            element: <SalesDetailPage kind="advance" />,
          },
          {
            path: "companies/:companyId/quotes",
            element: <SalesListPage kind="quote" />,
          },
          {
            path: "companies/:companyId/templates",
            element: <SalesListPage kind="template" />,
          },
          {
            path: "companies/:companyId/quotes/:salesId",
            element: <SalesDetailPage kind="quote" />,
          },
          {
            path: "companies/:companyId/templates/:salesId",
            element: <SalesDetailPage kind="template" />,
          },
          { path: "companies/:companyId/catalog", element: <CatalogPage /> },
          {
            path: "companies/:companyId/catalog/:catalogId",
            element: <CatalogDetailPage />,
          },
          {
            path: "companies/:companyId/invoices",
            element: <InvoiceDraftListPage />,
          },
          {
            path: "companies/:companyId/invoices/:invoiceId",
            element: <InvoiceDraftPage />,
          },
          { path: "companies/:companyId/contacts", element: <ContactsPage /> },
          {
            path: "companies/:companyId/contacts/:contactId",
            element: <ContactDetailPage />,
          },
          { path: "invoices/*", element: <UnavailablePage /> },
          { path: "login", element: <Navigate to="/workspace" replace /> },
          { path: "setup", element: <Navigate to="/workspace" replace /> },
          { path: "*", element: <NotFoundPage /> },
        ],
      },
    ],
  },
]);
export function ProductRoutes() {
  return <RouterProvider router={router} />;
}
