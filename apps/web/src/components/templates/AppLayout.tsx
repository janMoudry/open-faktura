import { Link } from "react-router";
import { AccountMenu } from "../molecules/AccountMenu";
import { Button } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
import type { ReactNode } from "react";
export function AppLayout({
  children,
  profileSwitcher,
  workspace = false,
  actions,
  email,
  name,
  onAccountSettings,
  onSmtp,
}: {
  children: ReactNode;
  profileSwitcher?: ReactNode;
  workspace?: boolean;
  actions?: ReactNode;
  email?: string;
  name?: string;
  onAccountSettings?: () => void;
  onSmtp?: () => void;
}) {
  const { t, language, setLanguage } = useAppTranslation();
  return (
    <div className={workspace ? "workspace-root" : "min-h-screen"}>
      <header
        className={`border-b border-border bg-surface ${workspace ? "workspace-header lg:sticky lg:z-30 lg:h-workspace-appbar" : ""}`}
      >
        <div
          className={
            workspace
              ? "workspace-appbar"
              : "mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-field px-group py-control"
          }
        >
          <Link
            to="/workspace"
            className={
              workspace ? "workspace-brand" : "text-lg font-bold tracking-tight"
            }
          >
            open<span className="text-primary">faktura</span>
          </Link>
          {workspace && (
            <div className="min-w-0 px-inline lg:px-field">
              {profileSwitcher}
            </div>
          )}
          <div
            className={`flex flex-wrap items-center gap-inline ${workspace ? "ml-auto px-field py-inline" : ""}`}
          >
            {workspace && email ? (
              <AccountMenu
                email={email}
                name={name}
                actions={actions}
                onSettings={onAccountSettings}
                onSmtp={onSmtp}
              />
            ) : (
              <>
                <div
                  role="group"
                  aria-label="Čeština / English"
                  className="flex gap-inline"
                >
                  {(["cs", "en"] as const).map((code) => (
                    <Button
                      key={code}
                      size="sm"
                      variant={language === code ? "secondary" : "ghost"}
                      aria-pressed={language === code}
                      lang={code}
                      onClick={() => setLanguage(code)}
                    >
                      {code === "cs" ? "Čeština" : "English"}
                    </Button>
                  ))}
                </div>
                {actions}
              </>
            )}
            {!workspace && (
              <span className="rounded-full bg-primary-soft px-3 py-1 text-xs font-medium text-primary">
                {t("version")}
              </span>
            )}
          </div>
        </div>
      </header>
      <main
        className={
          workspace ? "" : "mx-auto max-w-6xl px-group py-page md:py-20"
        }
      >
        {children}
      </main>
      {!workspace && (
        <footer className="mx-auto max-w-6xl border-t border-border px-6 py-6 text-xs text-muted">
          {t("footer")}
        </footer>
      )}
    </div>
  );
}
