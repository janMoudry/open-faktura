import { useEffect, useState } from "react";
import { Alert, Button, Checkbox, Dialog, Heading } from "@open-faktura/ui";
import {
  desktopUpdateStatus,
  desktopUpdateCheck,
  desktopUpdateInstall,
  isDesktopClient,
  type DesktopUpdateStatus,
  type DesktopCandidate,
} from "../../services/desktopUpdater";
import { useAppTranslation } from "../../locales";
export function DesktopUpdates() {
  const { t } = useAppTranslation();
  const [status, setStatus] = useState<DesktopUpdateStatus | null>(null),
    [candidate, setCandidate] = useState<DesktopCandidate | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(false),
    [current, setCurrent] = useState(false),
    [installed, setInstalled] = useState(false),
    [confirm, setConfirm] = useState(false),
    [saved, setSaved] = useState(false);
  useEffect(() => {
    if (!isDesktopClient()) return;
    let active = true;
    desktopUpdateStatus()
      .then((v) => {
        if (active) setStatus(v);
      })
      .catch(() => {
        if (active) setError(true);
      });
    return () => {
      active = false;
    };
  }, []);
  if (!isDesktopClient()) return null;
  async function check() {
    setBusy(true);
    setError(false);
    setCurrent(false);
    setInstalled(false);
    setCandidate(null);
    try {
      const v = await desktopUpdateCheck();
      setCandidate(v);
      setCurrent(!v);
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  async function install() {
    if (!candidate || !saved) return;
    setBusy(true);
    setError(false);
    try {
      await desktopUpdateInstall(candidate, saved);
      setInstalled(true);
      setCandidate(null);
      setConfirm(false);
    } catch {
      setCandidate(null);
      setConfirm(false);
      setError(true);
    } finally {
      setSaved(false);
      setBusy(false);
    }
  }
  return (
    <section className="workspace-panel p-field space-y-field">
      <Heading level={2}>{t("desktopUpdateTitle")}</Heading>
      <p className="text-sm text-muted">{t("desktopUpdateDescription")}</p>
      {error && (
        <Alert role="alert" tone="danger">
          {t("desktopUpdateFailed")}
        </Alert>
      )}
      {!status && !error && <p role="status">{t("cashLoading")}</p>}
      {status && (
        <>
          <p>
            {t("desktopUpdateVersion", { version: status.current_version })}
          </p>
          {!status.enabled ? (
            <p>{t("desktopUpdateDisabled")}</p>
          ) : (
            <Button disabled={busy} onClick={() => void check()}>
              {t("desktopUpdateCheck")}
            </Button>
          )}
        </>
      )}
      {busy && <p role="status">{t("desktopUpdateBusy")}</p>}
      {current && <p role="status">{t("desktopUpdateCurrent")}</p>}
      {installed && <p role="status">{t("desktopUpdateInstalled")}</p>}
      {candidate && (
        <div className="space-y-control">
          <p>{t("desktopUpdateAvailable", { version: candidate.version })}</p>
          {candidate.notes && (
            <pre className="whitespace-pre-wrap break-words text-sm">
              {candidate.notes}
            </pre>
          )}
          <Button
            disabled={busy}
            onClick={() => {
              setSaved(false);
              setConfirm(true);
            }}
          >
            {t("desktopUpdateInstall")}
          </Button>
        </div>
      )}
      <Dialog
        open={confirm}
        onClose={() => {
          if (!busy) setConfirm(false);
        }}
        title={t("desktopUpdateInstall")}
        description={t("desktopUpdateConfirmDescription", {
          version: candidate?.version ?? "",
        })}
      >
        <div className="space-y-field">
          <Checkbox
            label={t("desktopUpdateSaved")}
            checked={saved}
            disabled={busy}
            onChange={(e) => setSaved(e.target.checked)}
          />
          <Button
            disabled={!saved || busy || !candidate}
            onClick={() => void install()}
          >
            {t("desktopUpdateConfirm")}
          </Button>
        </div>
      </Dialog>
    </section>
  );
}
