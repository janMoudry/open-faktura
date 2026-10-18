import { useEffect, useState } from "react";
import { Alert, Button, Select, Switch } from "@open-faktura/ui";
import {
  dashboardWidgets,
  SalesFailure,
  type DashboardLayout,
  type DashboardWidget,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { apiUrl } from "../../services/status";
import { FinancialDashboard } from "./FinancialDashboard";
import { TaxDashboard } from "./TaxDashboard";
const defaults: DashboardWidget[] = [
  { kind: "finance", variant: "detailed" },
  { kind: "tax", variant: "detailed" },
];
export function DashboardWidgets({
  company,
  today,
  refresh,
  onExpired,
}: {
  company: string;
  today: string;
  refresh: number;
  onExpired: () => void;
}) {
  const { t } = useAppTranslation();
  const [value, setValue] = useState<DashboardLayout>({
      revision: 0,
      widgets: defaults,
    }),
    [loaded, setLoaded] = useState(false),
    [failed, setFailed] = useState(false),
    [busy, setBusy] = useState(false),
    [saved, setSaved] = useState(false),
    [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let active = true;
    dashboardWidgets(apiUrl)
      .then((v) => {
        if (active) {
          setValue(v);
          setLoaded(true);
          setFailed(false);
        }
      })
      .catch((e) => {
        if (active) {
          setFailed(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => {
      active = false;
    };
  }, [attempt, onExpired]);
  function change(widgets: DashboardWidget[]) {
    setValue({ ...value, widgets });
    setSaved(false);
  }
  async function save() {
    setBusy(true);
    setFailed(false);
    try {
      setValue(await dashboardWidgets(apiUrl, value));
      setSaved(true);
    } catch (e) {
      setFailed(true);
      if (e instanceof SalesFailure && e.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <details className="workspace-panel p-field">
        <summary className="cursor-pointer font-semibold">
          {t("widgetsCustomize")}
        </summary>
        <div className="space-y-field mt-field">
          <p className="text-sm text-muted">{t("widgetsScope")}</p>
          {(["finance", "tax"] as const).map((kind) => {
            const current = value.widgets.find((w) => w.kind === kind);
            return (
              <div key={kind} className="flex flex-wrap items-center gap-field">
                <Switch
                  label={t(
                    kind === "finance" ? "dashboardTitle" : "taxDashboardTitle",
                  )}
                  checked={!!current}
                  disabled={!loaded || busy}
                  onChange={(e) =>
                    change(
                      e.target.checked
                        ? [...value.widgets, { kind, variant: "detailed" }]
                        : value.widgets.filter((w) => w.kind !== kind),
                    )
                  }
                />
                {current && (
                  <Select
                    label={t("widgetsVariant")}
                    value={current.variant}
                    disabled={busy}
                    options={[
                      { value: "detailed", label: t("widgetsDetailed") },
                      { value: "compact", label: t("widgetsCompact") },
                    ]}
                    onChange={(v) =>
                      change(
                        value.widgets.map((w) =>
                          w.kind === kind
                            ? { ...w, variant: v as DashboardWidget["variant"] }
                            : w,
                        ),
                      )
                    }
                  />
                )}
              </div>
            );
          })}
          <div className="flex flex-wrap gap-control">
            <Button
              variant="secondary"
              disabled={!loaded || busy || value.widgets.length < 2}
              onClick={() => change([...value.widgets].reverse())}
            >
              {t("widgetsReverse")}
            </Button>
            <Button
              variant="secondary"
              disabled={!loaded || busy}
              onClick={() => change(defaults)}
            >
              {t("widgetsReset")}
            </Button>
            <Button loading={busy} disabled={!loaded} onClick={save}>
              {t("accountSave")}
            </Button>
          </div>
          {saved && <p role="status">{t("widgetsSaved")}</p>}
          {failed && (
            <Alert tone="warning" role="alert">
              {t("widgetsError")}{" "}
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setAttempt((v) => v + 1)}
              >
                {t("reloadCompany")}
              </Button>
            </Alert>
          )}
        </div>
      </details>
      {value.widgets.map((w) =>
        w.kind === "finance" ? (
          <FinancialDashboard
            key={`${company}:finance`}
            compact={w.variant === "compact"}
            company={company}
            today={today}
            refresh={refresh}
            onExpired={onExpired}
          />
        ) : (
          <TaxDashboard
            key={`${company}:tax`}
            compact={w.variant === "compact"}
            company={company}
            today={today}
            refresh={refresh}
            onExpired={onExpired}
          />
        ),
      )}
    </>
  );
}
