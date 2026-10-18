import { useState } from "react";
import { Alert, Badge, Button, Dialog, Field } from "@open-faktura/ui";
import type { ExportVersion } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useDocumentExports } from "../../hooks/useDocumentExports";
export function DocumentExports({
  company,
  target,
  editable,
  owner,
}: {
  company: string;
  target: ExportVersion;
  editable: boolean;
  owner: boolean;
}) {
  const { t, language } = useAppTranslation();
  const { history, busy, error, create, unlock, download } = useDocumentExports(
    company,
    target,
  );
  const [modal, setModal] = useState<"lock" | "unlock" | null>(null);
  const [reason, setReason] = useState("");
  return (
    <section className="border-t border-border pt-control">
      <details>
        <summary className="cursor-pointer text-sm font-medium">
          <span className="inline-flex flex-wrap items-center gap-inline">
            {t("documentExportsTitle")}
            {history?.locked_by_export && (
              <Badge>{t("documentExportsLocked")}</Badge>
            )}
            {error && (
              <span
                className="text-danger"
                aria-label={t("documentExportsError")}
                title={t("documentExportsError")}
              >
                !
              </span>
            )}
          </span>
        </summary>
        <div className="mt-field space-y-field">
          <p className="text-sm text-muted">{t("documentExportsHint")}</p>
          {error && <Alert tone="danger">{t("documentExportsError")}</Alert>}
          {history && (
            <>
              <div className="flex flex-wrap items-center gap-control">
                <Button
                  variant="secondary"
                  disabled={busy}
                  onClick={() => void create(false)}
                >
                  {t("documentExportsDownload")}
                </Button>
                {editable && !history.locked_by_export && (
                  <Button
                    variant="secondary"
                    disabled={busy}
                    onClick={() => setModal("lock")}
                  >
                    {t("documentExportsLock")}
                  </Button>
                )}
                {history.locked_by_export && (
                  <>
                    {owner && (
                      <Button
                        variant="secondary"
                        disabled={busy}
                        onClick={() => setModal("unlock")}
                      >
                        {t("documentExportsUnlock")}
                      </Button>
                    )}
                  </>
                )}
              </div>
              {history.exports.length > 0 && (
                <details>
                  <summary className="cursor-pointer">
                    {t("documentExportsHistory")}
                  </summary>
                  <ul className="space-y-control">
                    {history.exports.map((item) => (
                      <li
                        key={item.id}
                        className="flex flex-wrap items-center gap-control"
                      >
                        <span>
                          {new Intl.DateTimeFormat(
                            language === "cs" ? "cs-CZ" : "en-GB",
                            { dateStyle: "medium", timeStyle: "short" },
                          ).format(new Date(item.created_at))}{" "}
                          · {t("documentExportsRevision")}{" "}
                          {
                            item.targets.find(
                              (v) =>
                                v.kind === target.kind && v.id === target.id,
                            )?.revision
                          }
                        </span>
                        <Button
                          variant="secondary"
                          disabled={busy}
                          onClick={() => void download(item.id)}
                        >
                          {t("documentExportsDownload")}
                        </Button>
                      </li>
                    ))}
                  </ul>
                </details>
              )}
            </>
          )}
        </div>
      </details>
      <Dialog
        open={!!modal}
        onClose={() => {
          if (!busy) setModal(null);
        }}
        title={t(
          modal === "unlock" ? "documentExportsUnlock" : "documentExportsLock",
        )}
        description={t(
          modal === "unlock"
            ? "documentExportsUnlockHint"
            : "documentExportsLockHint",
        )}
      >
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            void (modal === "unlock" ? unlock(reason) : create(true)).then(
              (success) => {
                if (success) setModal(null);
              },
            );
          }}
        >
          {modal === "unlock" && (
            <Field
              label={t("documentExportsReason")}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              maxLength={500}
              required
              disabled={busy}
            />
          )}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setModal(null)}
            >
              {t("cancelAction")}
            </Button>
            <Button
              type="submit"
              disabled={busy || (modal === "unlock" && !reason.trim())}
            >
              {t(
                modal === "unlock"
                  ? "documentExportsUnlock"
                  : "documentExportsLock",
              )}
            </Button>
          </div>
        </form>
      </Dialog>
    </section>
  );
}
