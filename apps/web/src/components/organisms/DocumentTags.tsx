import { useState } from "react";
import {
  Alert,
  Badge,
  IconButton,
  Button,
  Field,
  Heading,
  Select,
  Dialog,
} from "@open-faktura/ui";
import type { TagTarget, DocumentTag } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useDocumentTags } from "../../hooks/useDocumentTags";
export function DocumentTags({
  company,
  editable,
  target,
}: {
  company: string;
  editable: boolean;
  target?: TagTarget;
}) {
  const { t } = useAppTranslation();
  const { all, selected, busy, error, execute } = useDocumentTags(
    company,
    target,
  );
  const [name, setName] = useState("");
  const [tag, setTag] = useState("");
  const [modal, setModal] = useState<{
    type: "rename" | "delete";
    tag: DocumentTag;
  } | null>(null);
  const [rename, setRename] = useState("");
  const visible = target ? selected : all;
  if (target) {
    const available = all.filter(
      (item) => !selected.some((s) => s.id === item.id),
    );
    return (
      <section
        className="flex flex-wrap items-center gap-control border-t border-border pt-control"
        aria-label={t("documentTagsTitle")}
      >
        <span className="text-sm font-medium text-muted">
          {t("documentTagsTitle")}
        </span>
        <ul className="flex flex-wrap items-center gap-inline">
          {selected.map((item) => (
            <li key={item.id}>
              <Badge>
                <span className="inline-flex items-center gap-inline">
                  {item.name}
                  {editable && (
                    <IconButton
                      variant="ghost"
                      size="sm"
                      icon="close"
                      label={`${t("documentTagsDetach")}: ${item.name}`}
                      disabled={busy}
                      onClick={() =>
                        void execute({
                          request_id: crypto.randomUUID(),
                          action: "detach",
                          tag_id: item.id,
                          target,
                        })
                      }
                    />
                  )}
                </span>
              </Badge>
            </li>
          ))}
        </ul>
        {editable && (
          <Select
            aria-label={t("documentTagsAttach")}
            className="w-full sm:w-auto"
            value=""
            disabled={busy || !available.length}
            options={[
              { value: "", label: t("documentTagsChoose") },
              ...available.map((item) => ({
                value: item.id,
                label: item.name,
              })),
            ]}
            onChange={(tag_id) => {
              if (tag_id)
                void execute({
                  request_id: crypto.randomUUID(),
                  action: "attach",
                  tag_id,
                  target,
                });
            }}
          />
        )}
        {!editable && !selected.length && (
          <span className="text-sm text-muted">—</span>
        )}
        {error && <Alert tone="danger">{t("documentTagsError")}</Alert>}
      </section>
    );
  }
  return (
    <section className="mt-group space-y-field border-t border-border pt-group">
      <Heading level={2}>{t("documentTagsTitle")}</Heading>
      {error && <Alert tone="danger">{t("documentTagsError")}</Alert>}
      <ul className="flex flex-wrap gap-control">
        {visible.map((item) => (
          <li
            className="flex items-center gap-control rounded-control border border-border p-control"
            key={item.id}
          >
            <span>
              {item.name}
              {!target && ` (${item.usage_count})`}
            </span>
            {editable && (
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() => {
                  if (target)
                    void execute({
                      request_id: crypto.randomUUID(),
                      action: "detach",
                      tag_id: item.id,
                      target,
                    });
                  else setModal({ type: "delete", tag: item });
                }}
              >
                {t(target ? "documentTagsDetach" : "documentTagsDelete")}
              </Button>
            )}
            {editable && !target && (
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() => {
                  setRename(item.name);
                  setModal({ type: "rename", tag: item });
                }}
              >
                {t("documentTagsRename")}
              </Button>
            )}
          </li>
        ))}
      </ul>
      {editable &&
        (target ? (
          <form
            className="flex flex-wrap items-end gap-field"
            onSubmit={(e) => {
              e.preventDefault();
              if (tag)
                void execute({
                  request_id: crypto.randomUUID(),
                  action: "attach",
                  tag_id: tag,
                  target,
                });
            }}
          >
            <Select
              label={t("documentTagsTitle")}
              value={tag}
              onChange={setTag}
              disabled={busy}
              options={[
                { value: "", label: t("documentTagsChoose") },
                ...all
                  .filter((i) => !selected.some((s) => s.id === i.id))
                  .map((i) => ({ value: i.id, label: i.name })),
              ]}
            />
            <Button type="submit" disabled={busy || !tag}>
              {t("documentTagsAttach")}
            </Button>
          </form>
        ) : (
          <form
            className="flex flex-wrap items-end gap-field"
            onSubmit={(e) => {
              e.preventDefault();
              if (name.trim())
                void execute({
                  request_id: crypto.randomUUID(),
                  action: "create",
                  name,
                });
            }}
          >
            <Field
              label={t("documentTagsName")}
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={80}
              required
              disabled={busy}
            />
            <Button type="submit" disabled={busy || !name.trim()}>
              {t("documentTagsCreate")}
            </Button>
          </form>
        ))}
      <Dialog
        open={!!modal}
        onClose={() => {
          if (!busy) setModal(null);
        }}
        title={t(
          modal?.type === "rename"
            ? "documentTagsRename"
            : "documentTagsDelete",
        )}
        description={
          modal?.type === "delete"
            ? t("documentTagsDeleteConfirm")
            : t("documentTagsName")
        }
      >
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            if (!modal) return;
            void execute(
              modal.type === "rename"
                ? {
                    request_id: crypto.randomUUID(),
                    action: "rename",
                    tag_id: modal.tag.id,
                    revision: modal.tag.revision,
                    name: rename,
                  }
                : {
                    request_id: crypto.randomUUID(),
                    action: "delete",
                    tag_id: modal.tag.id,
                    revision: modal.tag.revision,
                  },
            ).then((success) => {
              if (success) setModal(null);
            });
          }}
        >
          {modal?.type === "rename" && (
            <Field
              label={t("documentTagsName")}
              value={rename}
              onChange={(e) => setRename(e.target.value)}
              maxLength={80}
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
              disabled={busy || (modal?.type === "rename" && !rename.trim())}
            >
              {t(
                modal?.type === "rename"
                  ? "documentTagsRename"
                  : "documentTagsDelete",
              )}
            </Button>
          </div>
        </form>
      </Dialog>
    </section>
  );
}
