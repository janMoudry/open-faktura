import { useAppTranslation } from "../locales";
import { Heading } from "@open-faktura/ui";
import { Button } from "@open-faktura/ui";
import { AppLayout } from "../components/templates/AppLayout";
import { ConnectionStatus } from "../components/molecules/ConnectionStatus";
import { useStatus } from "../hooks/useStatus";
export function HomePage() {
  const { t } = useAppTranslation();
  const status = useStatus();
  return (
    <AppLayout>
      <p className="mb-4 text-xs font-semibold uppercase tracking-widest text-primary">
        {t("tagline")}
      </p>
      <div className="grid gap-12 md:grid-cols-2 md:gap-20">
        <section>
          <Heading level={1} size="page">
            {t("foundation")}
            <br />
            {t("ahead")}
          </Heading>
          <p className="mt-6 max-w-md text-lg leading-relaxed text-muted">
            {t("homeIntro")}
          </p>
          <div className="mt-10 border-l-2 border-primary pl-5">
            <Heading level={2} size="section">
              {t("firstInvoice")}
            </Heading>
            <p className="mt-2 text-sm leading-relaxed text-muted">
              {t("development")}
            </p>
          </div>
        </section>
        <section
          aria-labelledby="connection-heading"
          className="self-start rounded-panel border border-border bg-surface p-7"
        >
          <p className="text-xs uppercase tracking-widest text-muted">
            {t("environment")}
          </p>
          <Heading
            level={2}
            size="section"
            id="connection-heading"
            className="mt-2"
          >
            {t("connectionHeading")}
          </Heading>
          <div className="my-4" aria-live="polite">
            <ConnectionStatus
              label={t("server")}
              connected={status.api}
              loading={status.loading}
            />
            <ConnectionStatus
              label={t("database")}
              connected={status.database}
              loading={status.loading}
            />
            {status.error && (
              <p
                role="alert"
                className="mt-4 text-sm leading-relaxed text-danger"
              >
                {t("connectionFailed")}
              </p>
            )}
          </div>
          <Button onClick={status.refresh} disabled={status.loading}>
            {t("checkConnection")}
            <span aria-hidden="true">↗</span>
          </Button>
        </section>
      </div>
    </AppLayout>
  );
}
