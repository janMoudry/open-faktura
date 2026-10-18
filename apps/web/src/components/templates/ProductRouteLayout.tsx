import { Suspense, useEffect, useState } from "react";
import {
  Outlet,
  useLocation,
  useNavigate,
  useOutletContext,
} from "react-router";
import { Alert, Button, Dialog, Icon } from "@open-faktura/ui";
import { logout } from "@open-faktura/api-client";
import type { SessionContext } from "../../routes/ProductRoutes";
import { useAppTranslation, type AppKey } from "../../locales";
import {
  CompanyWorkspaceContext,
  useCompanyWorkspace,
} from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
import { authMessage } from "../../utils/auth-error";
import { SmtpSettingsDialog } from "../organisms/SmtpSettingsDialog";
import { AccountSettingsDialog } from "../organisms/AccountSettingsDialog";
import { InboxNotifications } from "../organisms/InboxNotifications";
import { AppLayout } from "./AppLayout";
import { ProductWorkspace } from "./ProductWorkspace";
import { ProfileSwitcher } from "../molecules/ProfileSwitcher";

export function WorkspaceLayout() {
  const session = useOutletContext<SessionContext>();
  // Reset all private workspace state only when the identity changes.
  return <AuthenticatedWorkspace key={session.user.id} session={session} />;
}
function AuthenticatedWorkspace({ session }: { session: SessionContext }) {
  const { t } = useAppTranslation();
  const workspace = useCompanyWorkspace(session.user.id, session.expire);
  const { companies, activeProfile, busy, start, blocker } = workspace;
  const [smtpOpen, setSmtpOpen] = useState(false);
  const [accountOpen, setAccountOpen] = useState(false);
  const [signingOut, setSigningOut] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const homeProfile =
    pathname.match(/^\/companies\/([^/]+)(?:\/|$)/)?.[1] ?? activeProfile;
  useEffect(() => {
    window.scrollTo(0, 0);
  }, [pathname]);
  return (
    <CompanyWorkspaceContext.Provider value={workspace}>
      <AppLayout
        workspace
        homeTo={
          homeProfile ? `/companies/${homeProfile}/overview` : "/workspace"
        }
        notifications={
          homeProfile ? (
            <InboxNotifications
              key={homeProfile}
              companyId={homeProfile}
              onExpired={session.expire}
            />
          ) : undefined
        }
        email={session.user.email}
        name={session.user.name}
        onSmtp={
          session.user.installation_admin ? () => setSmtpOpen(true) : undefined
        }
        onAccountSettings={() => setAccountOpen(true)}
        profileSwitcher={
          <ProfileSwitcher
            profiles={companies.map((c) => ({
              value: c.id,
              label: c.settings.name || t("untitledCompany"),
            }))}
            selected={activeProfile}
            busy={busy}
            onCreate={start}
            onSelect={(id) => {
              void navigate(
                `/companies/${id}${pathname.includes("/bank") ? "/bank" : pathname.includes("/receipts") ? "/receipts" : pathname.includes("/expense-payments") ? "/expense-payments" : pathname.includes("/expense-overview") ? "/expense-overview" : pathname.includes("/received-invoices") ? "/received-invoices" : pathname.includes("/quotes") ? "/quotes" : pathname.includes("/templates") ? "/templates" : pathname.includes("/catalog") ? "/catalog" : pathname.includes("/contacts") ? "/contacts" : pathname.includes("/invoices") ? "/invoices" : ""}`,
              );
            }}
          />
        }
        actions={
          <>
            {error && (
              <Alert tone="danger" role="alert">
                {t(error)}
              </Alert>
            )}
            <Button
              variant="ghost"
              className="account-menu-item"
              loading={signingOut}
              onClick={async () => {
                setSigningOut(true);
                setError("");
                try {
                  await logout(apiUrl, AbortSignal.timeout(10000));
                  session.expire();
                  void navigate("/login", { replace: true });
                } catch (error) {
                  setError(authMessage(error));
                } finally {
                  setSigningOut(false);
                }
              }}
            >
              {!signingOut && <Icon name="logout" />}
              {t("signOut")}
            </Button>
          </>
        }
      >
        <ProductWorkspace
          selected={activeProfile}
          canCreate={
            !!workspace.company &&
            workspace.company.id === activeProfile &&
            workspace.company.role !== "viewer"
          }
        >
          <Suspense fallback={<p role="status">{t("companiesLoading")}</p>}>
            <Outlet />
          </Suspense>
        </ProductWorkspace>
        {smtpOpen && (
          <SmtpSettingsDialog
            email={session.user.email}
            onClose={() => setSmtpOpen(false)}
            onExpired={session.expire}
          />
        )}
        {accountOpen && (
          <AccountSettingsDialog
            user={session.user}
            onClose={() => setAccountOpen(false)}
            onSaved={session.setUser}
          />
        )}
        <Dialog
          open={blocker.state === "blocked"}
          onClose={() => blocker.reset?.()}
          title={t("unsavedProfile")}
          description={t("unsavedProfileHint")}
        >
          <div className="flex flex-wrap justify-end gap-control">
            <Button variant="secondary" onClick={() => blocker.reset?.()}>
              {t("stayHere")}
            </Button>
            <Button variant="danger" onClick={() => blocker.proceed?.()}>
              {t("discardAndSwitch")}
            </Button>
          </div>
        </Dialog>
      </AppLayout>
    </CompanyWorkspaceContext.Provider>
  );
}
