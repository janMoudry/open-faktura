import { Link } from "react-router";
import { Button, Heading, Icon } from "@open-faktura/ui";
import { useAppTranslation, type AppKey } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";

import { useStatus } from "../hooks/useStatus";
import { ConnectionStatus } from "../components/molecules/ConnectionStatus";

const guides: { title: AppKey; text: AppKey; path: string }[] = [
  { title: "helpInvoices", text: "helpInvoicesText", path: "/invoices" },
  { title: "helpPayments", text: "helpPaymentsText", path: "/bank" },
  { title: "helpInbox", text: "helpInboxText", path: "/inbox" },
  { title: "helpMigration", text: "helpMigrationText", path: "/imports" },
  { title: "helpReports", text: "helpReportsText", path: "/reports" },
  { title: "helpProfiles", text: "helpProfilesText", path: "" },
];
const docs: { label: AppKey; path: string }[] = [
  {
    label: "importAdapterGuide",
    path: "blob/main/docs/INTEGRATION_ADAPTERS.md",
  },
  { label: "helpInstallation", path: "blob/main/docs/INSTALLATION.md" },
  { label: "helpFeatureStatus", path: "blob/main/docs/FEATURE_CHECKLIST.md" },
  { label: "helpUpdates", path: "releases" },
  { label: "helpReportIssue", path: "issues" },
];
export function HelpPage() {
  const { t } = useAppTranslation();
  const { activeProfile } = useWorkspace();
  const status = useStatus();
  return (
    <div className="space-y-group">
      <header className="space-y-control">
        <Heading level={1} size="page">
          {t("helpTitle")}
        </Heading>
        <p className="text-sm text-muted">{t("helpIntro")}</p>
      </header>
      <div className="grid gap-field lg:grid-cols-2">
        {guides.map((guide) => (
          <section
            key={guide.title}
            className="rounded-panel border border-border bg-surface p-field space-y-control"
          >
            <Heading level={2} size="section">
              {t(guide.title)}
            </Heading>
            <p className="text-sm leading-relaxed text-muted">
              {t(guide.text)}
            </p>
            <Link
              className="inline-flex items-center gap-inline text-sm font-medium text-primary underline underline-offset-4"
              to={
                activeProfile
                  ? `/companies/${activeProfile}${guide.path}`
                  : "/workspace"
              }
            >
              {t("helpOpenModule")}
              <Icon name="chevronRight" />
            </Link>
          </section>
        ))}
      </div>
      <section className="rounded-panel border border-border bg-surface p-field">
        <Heading level={2} size="section">
          {t("helpConnection")}
        </Heading>
        <ConnectionStatus
          label={t("helpServer")}
          connected={status.api}
          loading={status.loading}
        />
        <ConnectionStatus
          label={t("database")}
          connected={status.database}
          loading={status.loading}
        />
        <Button
          variant="secondary"
          loading={status.loading}
          onClick={status.refresh}
        >
          {t("helpCheckConnection")}
        </Button>
      </section>
      <section className="rounded-panel border border-border bg-surface p-field space-y-field">
        <Heading level={2} size="section">
          {t("helpOperations")}
        </Heading>
        <p className="text-sm text-muted">{t("helpOperationsText")}</p>
        <div className="flex flex-wrap gap-field">
          {docs.map((doc) => (
            <a
              key={doc.label}
              href={`https://github.com/janMoudry/open-faktura/${doc.path}`}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-inline text-sm font-medium text-primary underline underline-offset-4"
            >
              {t(doc.label)}
              <Icon name="external" />
              <span className="sr-only">{t("helpNewTab")}</span>
            </a>
          ))}
        </div>
        <p className="text-xs text-muted">{t("helpIssuePrivacy")}</p>
      </section>
    </div>
  );
}
