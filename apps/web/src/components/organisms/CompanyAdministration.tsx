import type {
  AdministrationAction,
  AdministrationSnapshot,
} from "@open-faktura/api-client";
import { useState } from "react";
import {
  Alert,
  Button,
  Field,
  Heading,
  Select,
  Dialog,
} from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
import { useCompanyAdministration } from "../../hooks/useCompanyAdministration";
export function CompanyAdministration({
  company,
  owner,
  onExpired,
}: {
  company?: string;
  owner: boolean;
  onExpired: () => void;
}) {
  const { t } = useAppTranslation();
  const { data, trash, busy, error, execute, download } =
    useCompanyAdministration(company, onExpired);
  const [user, setUser] = useState("");
  const [role, setRole] = useState<"viewer" | "editor">("viewer");
  const [pending, setPending] = useState<{
    snapshot: AdministrationSnapshot;
    action: AdministrationAction;
    type: "transfer" | "remove" | "leave";
  } | null>(null);
  return (
    <section className="mt-group space-y-field border-t border-border pt-group">
      <Heading level={2}>
        {t(company ? "administrationTitle" : "administrationTrash")}
      </Heading>
      {error && <Alert tone="danger">{t("administrationError")}</Alert>}
      {data && (
        <>
          <p className="text-sm text-muted-foreground">
            {t("administrationHistoryHint")}
          </p>
          <ul className="space-y-control">
            {data.members.map((m) => (
              <li
                key={m.user_id}
                className="flex flex-wrap items-center gap-control"
              >
                <span>
                  {m.name ? `${m.name} (${m.email})` : m.email}:{" "}
                  {t(
                    m.role === "owner"
                      ? "administrationOwner"
                      : m.role === "editor"
                        ? "administrationEditor"
                        : "administrationViewer",
                  )}
                </span>
                {owner && m.role !== "owner" && (
                  <>
                    <Button
                      variant="secondary"
                      disabled={busy}
                      onClick={() =>
                        void execute(data, {
                          action: "set_role",
                          user_id: m.user_id,
                          role: m.role === "editor" ? "viewer" : "editor",
                        })
                      }
                    >
                      {t("administrationChangeRole")}
                    </Button>
                    <Button
                      variant="secondary"
                      disabled={busy}
                      onClick={() => {
                        setPending({
                          snapshot: data,
                          action: { action: "transfer", user_id: m.user_id },
                          type: "transfer",
                        });
                      }}
                    >
                      {t("administrationTransfer")}
                    </Button>
                    <Button
                      variant="secondary"
                      disabled={busy}
                      onClick={() => {
                        setPending({
                          snapshot: data,
                          action: {
                            action: "remove_member",
                            user_id: m.user_id,
                          },
                          type: "remove",
                        });
                      }}
                    >
                      {t("administrationRemove")}
                    </Button>
                  </>
                )}
              </li>
            ))}
          </ul>
          {owner && (
            <div className="space-y-control">
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() => void download()}
              >
                {t("administrationArchive")}
              </Button>
              <p className="text-sm text-muted-foreground">
                {t("administrationArchiveHint")}
              </p>
            </div>
          )}
          {owner ? (
            <form
              className="flex flex-wrap items-end gap-field"
              onSubmit={(e) => {
                e.preventDefault();
                if (user.trim())
                  void execute(data, {
                    action: "add_member",
                    email: user.trim(),
                    role,
                  });
              }}
            >
              <Field
                label={t("email")}
                type="email"
                value={user}
                onChange={(e) => setUser(e.target.value)}
                required
                disabled={busy}
              />
              <Select
                label={t("administrationRole")}
                value={role}
                onChange={(e) => setRole(e as "viewer" | "editor")}
                options={[
                  { value: "viewer", label: t("administrationViewer") },
                  { value: "editor", label: t("administrationEditor") },
                ]}
                disabled={busy}
              />
              <Button type="submit" disabled={busy || !user.trim()}>
                {t("administrationAdd")}
              </Button>
            </form>
          ) : (
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => {
                setPending({
                  snapshot: data,
                  action: { action: "leave" },
                  type: "leave",
                });
              }}
            >
              {t("administrationLeave")}
            </Button>
          )}
        </>
      )}
      {!company &&
        (trash.length ? (
          <ul className="space-y-control">
            {trash.map((c) => (
              <li key={c.company_id} className="flex items-center gap-field">
                <span>
                  {t("administrationProfile")} {c.company_id}
                </span>
                <Button
                  disabled={busy}
                  onClick={() => void execute(c, { action: "restore" })}
                >
                  {t("administrationRestore")}
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">
            {t("administrationTrashEmpty")}
          </p>
        ))}
      <Dialog
        open={!!pending}
        onClose={() => {
          if (!busy) setPending(null);
        }}
        title={t(
          pending?.type === "transfer"
            ? "administrationTransfer"
            : pending?.type === "remove"
              ? "administrationRemove"
              : "administrationLeave",
        )}
        description={t(
          pending?.type === "transfer"
            ? "administrationTransferConfirm"
            : pending?.type === "remove"
              ? "administrationRemoveConfirm"
              : "administrationLeaveConfirm",
        )}
      >
        <div className="flex justify-end gap-control">
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => setPending(null)}
          >
            {t("cancelAction")}
          </Button>
          <Button
            disabled={busy}
            onClick={() => {
              if (pending)
                void execute(pending.snapshot, pending.action).then(() =>
                  setPending(null),
                );
            }}
          >
            {t(
              pending?.type === "transfer"
                ? "administrationTransfer"
                : pending?.type === "remove"
                  ? "administrationRemove"
                  : "administrationLeave",
            )}
          </Button>
        </div>
      </Dialog>
    </section>
  );
}
