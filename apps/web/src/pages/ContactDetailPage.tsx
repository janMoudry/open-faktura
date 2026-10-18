import { DocumentTags } from "../components/organisms/DocumentTags";
import { useEffect, useRef, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Alert,
  Badge,
  Button,
  Dialog,
  Heading,
  Icon,
  ActionMenu,
} from "@open-faktura/ui";
import {
  ContactFailure,
  createContact,
  getContact,
  updateContact,
  type Contact,
  type ContactData,
} from "@open-faktura/api-client";
import { VatRegistryLookup } from "../components/organisms/VatRegistryLookup";
import { ContactOverview } from "../components/organisms/ContactOverview";
import { ContactFields } from "../components/organisms/ContactFields";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { contactError, emptyContact } from "../utils/contact";
import { useAresLookup } from "../hooks/useAresLookup";
import { apiUrl } from "../services/status";
export function ContactDetailPage() {
  const { companyId = "", contactId = "" } = useParams();
  return (
    <ContactEditor
      key={`${companyId}:${contactId}`}
      companyId={companyId}
      contactId={contactId}
    />
  );
}
function ContactEditor({
  companyId,
  contactId,
}: {
  companyId: string;
  contactId: string;
}) {
  const { t } = useAppTranslation();
  const { company, setPageDirty, onExpired, navigateAfterSave } =
    useWorkspace();
  const [params] = useSearchParams();
  const creating = contactId === "new";
  const [editRequested, setEditing] = useState(
    creating || params.get("edit") === "true",
  );
  const [contact, setContact] = useState<Contact | null>(null);
  const [data, setData] = useState<ContactData>(emptyContact);
  const ares = useAresLookup(companyId, setData, onExpired);
  const [loading, setLoading] = useState(!creating);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const [fields, setFields] = useState<string[]>([]);
  const [saved, setSaved] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [action, setAction] = useState<
    "archive" | "restore" | "reload" | "discard" | null
  >(null);
  const requestId = useRef(crypto.randomUUID());
  const canWrite = company?.id === companyId && company.role !== "viewer";
  const editing = editRequested && canWrite && !contact?.archived;
  function finishEditing() {
    setEditing(false);
    if (params.has("edit"))
      navigateAfterSave(`/companies/${companyId}/contacts/${contactId}`);
  }
  const dirty =
    JSON.stringify(data) !== JSON.stringify(contact?.data ?? emptyContact());
  useEffect(() => {
    setPageDirty(dirty);
    return () => setPageDirty(false);
  }, [dirty, setPageDirty]);
  const report = (error: unknown) => {
    if (error instanceof ContactFailure && error.status === 401) {
      onExpired();
      return;
    }
    setError(contactError(error));
    setFields(error instanceof ContactFailure ? error.fields : []);
  };
  useEffect(() => {
    if (creating) return;
    const controller = new AbortController();
    setLoading(true);
    setError("");
    setSaved(false);
    getContact(apiUrl, companyId, contactId, controller.signal)
      .then((contact) => {
        if (!controller.signal.aborted) {
          setContact(contact);
          setData(contact.data);
        }
      })
      .catch((error) => {
        if (!controller.signal.aborted) report(error);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [companyId, contactId, creating, attempt]);
  async function save(archived = contact?.archived ?? false) {
    if (busy || ares.loading || !canWrite) return;
    setBusy(true);
    setError("");
    setFields([]);
    setSaved(false);
    const normalized = {
      ...data,
      recipients: data.recipients.map((v) => v.trim()).filter(Boolean),
    };
    try {
      const result = creating
        ? await createContact(apiUrl, companyId, {
            request_id: requestId.current,
            data: normalized,
          })
        : await updateContact(apiUrl, companyId, contactId, {
            revision: contact!.revision,
            archived,
            data: normalized,
          });
      setContact(result);
      setData(result.data);
      setPageDirty(false);
      setSaved(true);
      finishEditing();
      setAction(null);
      if (creating) {
        navigateAfterSave(`/companies/${companyId}/contacts/${result.id}`);
      }
    } catch (error) {
      report(error);
    } finally {
      setBusy(false);
    }
  }
  if (loading) return <p role="status">{t("contactsLoading")}</p>;
  return (
    <div className="space-y-field">
      <Link
        className="inline-flex items-center gap-inline text-sm text-muted hover:underline"
        to={`/companies/${companyId}/contacts`}
      >
        <Icon name="back" />
        {t("navContacts")}
      </Link>
      <div className="workspace-page-heading flex flex-wrap items-center justify-between gap-field">
        <div className="flex flex-wrap items-center gap-field">
          <Heading level={1} size="page">
            {creating
              ? t("contactNew")
              : (contact?.data.name ?? t("contactDetail"))}
          </Heading>
          {contact && !editing && canWrite && !contact.archived && (
            <Button
              size="sm"
              onClick={() => {
                setEditing(true);
                setSaved(false);
              }}
            >
              <Icon name="edit" />
              {t("contactEdit")}
            </Button>
          )}
          {contact?.archived && (
            <Badge tone="warning">{t("contactArchived")}</Badge>
          )}
        </div>
        {contact && !editing && (
          <div className="flex flex-wrap items-center gap-inline">
            <Link
              className="ui-button ui-button-ghost ui-button-sm"
              to={`/companies/${companyId}/invoices?${new URLSearchParams({ customer: contact.id, customerName: contact.data.name })}`}
            >
              {t("contactRelated")}
            </Link>
            {canWrite && (
              <ActionMenu
                label={t("contactActions")}
                disabled={busy || dirty}
                items={[
                  {
                    id: "archive",
                    label: t(
                      contact.archived ? "contactRestore" : "contactArchive",
                    ),
                    onAction: () =>
                      setAction(contact.archived ? "restore" : "archive"),
                  },
                ]}
              />
            )}
          </div>
        )}
      </div>
      {error && (
        <Alert tone="danger" role="alert">
          <p>{t(error)}</p>
          {error === "contactConflict" && (
            <Button
              variant="secondary"
              className="mt-field"
              onClick={() => setAction("reload")}
            >
              {t("reloadCompany")}
            </Button>
          )}
          {!contact && !creating && (
            <Button
              variant="secondary"
              className="mt-field"
              onClick={() => setAttempt((v) => v + 1)}
            >
              {t("retry")}
            </Button>
          )}
        </Alert>
      )}
      {saved && (
        <Alert tone="success" role="status">
          {t("contactSaved")}
        </Alert>
      )}
      {contact && (
        <p className="flex items-center gap-inline text-sm text-muted">
          <Icon name="info" />
          {t("contactSnapshotHint")}
        </p>
      )}
      <VatRegistryLookup
        key={`${companyId}:${data.vat_number}`}
        company={companyId}
        vatNumber={data.vat_number}
        onExpired={onExpired}
      />
      {!editing && contact && <ContactOverview data={contact.data} />}
      {!canWrite && contact && (
        <p className="text-sm text-muted">{t("contactReadOnly")}</p>
      )}
      {editing && (creating || contact) && (
        <form
          className="space-y-field"
          noValidate
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <ContactFields
            footer={
              canWrite &&
              !contact?.archived && (
                <div className="profile-actions">
                  {creating ? (
                    <Link
                      className="ui-button ui-button-secondary ui-button-md"
                      to={`/companies/${companyId}/contacts`}
                    >
                      {t("cancelAction")}
                    </Link>
                  ) : (
                    <Button
                      variant="secondary"
                      disabled={busy || ares.loading}
                      onClick={() =>
                        dirty ? setAction("discard") : finishEditing()
                      }
                    >
                      {t("cancelAction")}
                    </Button>
                  )}
                  <Button
                    type="submit"
                    loading={busy}
                    disabled={!dirty || ares.loading}
                  >
                    {t(creating ? "contactCreate" : "accountSave")}
                  </Button>
                </div>
              )
            }
            value={data}
            disabled={!canWrite || busy || !!contact?.archived}
            fields={fields}
            ares={{
              ...ares,
              lookup: () => {
                if (canWrite && !busy && !contact?.archived)
                  void ares.lookup(data);
              },
            }}
            onChange={(next) => {
              if (next.registration_number !== data.registration_number)
                ares.cancel();
              setData(next);
              setSaved(false);
            }}
          />
          {!canWrite && (
            <p className="p-form-inset text-sm text-muted">
              {t("contactReadOnly")}
            </p>
          )}
        </form>
      )}
      <Dialog
        open={action !== null}
        onClose={() => {
          if (!busy) setAction(null);
        }}
        title={t(
          action === "discard"
            ? "contactDiscard"
            : action === "reload"
              ? "reloadCompany"
              : action === "restore"
                ? "contactRestore"
                : "contactArchive",
        )}
        description={t(
          action === "discard"
            ? "contactDiscardHint"
            : action === "reload"
              ? "contactReloadHint"
              : "contactArchiveHint",
        )}
      >
        {error && (
          <Alert tone="danger" role="alert">
            {t(error)}
          </Alert>
        )}
        <div className="mt-group flex justify-end gap-control">
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => setAction(null)}
          >
            {t("cancelAction")}
          </Button>
          <Button
            loading={busy}
            onClick={() => {
              if (action === "discard") {
                ares.cancel();
                setData(contact!.data);
                setFields([]);
                setError("");
                setSaved(false);
                setAction(null);
                finishEditing();
              } else if (action === "reload") {
                setAction(null);
                setAttempt((v) => v + 1);
              } else void save(action === "archive");
            }}
          >
            {t(
              action === "discard"
                ? "contactDiscard"
                : action === "reload"
                  ? "reloadCompany"
                  : action === "restore"
                    ? "contactRestore"
                    : "contactArchive",
            )}
          </Button>
        </div>
      </Dialog>
      {contact && (
        <DocumentTags
          key={`contact-tags-${contact.id}`}
          company={companyId}
          editable={company?.role !== "viewer"}
          target={{ kind: "contact", id: contact.id }}
        />
      )}
    </div>
  );
}
