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
  CatalogFailure,
  createCatalog,
  getCatalog,
  updateCatalog,
  type Catalog,
  type CatalogData,
} from "@open-faktura/api-client";
import { CatalogOverview } from "../components/organisms/CatalogOverview";
import { CatalogFields } from "../components/organisms/CatalogFields";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { catalogError, emptyCatalog } from "../utils/catalog";
import { apiUrl } from "../services/status";
export function CatalogDetailPage() {
  const { companyId = "", catalogId = "" } = useParams();
  return (
    <CatalogEditor
      key={`${companyId}:${catalogId}`}
      companyId={companyId}
      catalogId={catalogId}
    />
  );
}
function CatalogEditor({
  companyId,
  catalogId,
}: {
  companyId: string;
  catalogId: string;
}) {
  const { t } = useAppTranslation();
  const { company, setPageDirty, onExpired, navigateAfterSave } =
    useWorkspace();
  const [params] = useSearchParams();
  const creating = catalogId === "new";
  const [editRequested, setEditing] = useState(
    creating || params.get("edit") === "true",
  );
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [data, setData] = useState<CatalogData>(emptyCatalog);
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
  const editing = editRequested && canWrite && !catalog?.archived;
  function finishEditing() {
    setEditing(false);
    if (params.has("edit"))
      navigateAfterSave(`/companies/${companyId}/catalog/${catalogId}`);
  }
  const dirty =
    JSON.stringify(data) !== JSON.stringify(catalog?.data ?? emptyCatalog());
  useEffect(() => {
    setPageDirty(dirty);
    return () => setPageDirty(false);
  }, [dirty, setPageDirty]);
  const report = (error: unknown) => {
    if (error instanceof CatalogFailure && error.status === 401) {
      onExpired();
      return;
    }
    setError(catalogError(error));
    setFields(error instanceof CatalogFailure ? error.fields : []);
  };
  useEffect(() => {
    if (creating) return;
    const controller = new AbortController();
    setLoading(true);
    setError("");
    setSaved(false);
    getCatalog(apiUrl, companyId, catalogId, controller.signal)
      .then((catalog) => {
        if (!controller.signal.aborted) {
          setCatalog(catalog);
          setData(catalog.data);
        }
      })
      .catch((error) => {
        if (!controller.signal.aborted) report(error);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [companyId, catalogId, creating, attempt]);
  async function save(archived = catalog?.archived ?? false) {
    if (busy || !canWrite) return;
    setBusy(true);
    setError("");
    setFields([]);
    setSaved(false);
    const normalized = data;
    try {
      const result = creating
        ? await createCatalog(apiUrl, companyId, {
            request_id: requestId.current,
            data: normalized,
          })
        : await updateCatalog(apiUrl, companyId, catalogId, {
            revision: catalog!.revision,
            archived,
            data: normalized,
          });
      setCatalog(result);
      setData(result.data);
      setPageDirty(false);
      setSaved(true);
      finishEditing();
      setAction(null);
      if (creating) {
        navigateAfterSave(`/companies/${companyId}/catalog/${result.id}`);
      }
    } catch (error) {
      report(error);
    } finally {
      setBusy(false);
    }
  }
  if (loading) return <p role="status">{t("catalogsLoading")}</p>;
  return (
    <div className="space-y-field">
      <Link
        className="inline-flex items-center gap-inline text-sm text-muted hover:underline"
        to={`/companies/${companyId}/catalog`}
      >
        <Icon name="back" />
        {t("catalogList")}
      </Link>
      <div className="workspace-page-heading flex flex-wrap items-center justify-between gap-field">
        <div className="flex flex-wrap items-center gap-field">
          <Heading level={1} size="page">
            {creating
              ? t("catalogNew")
              : (catalog?.data.name ?? t("catalogDetail"))}
          </Heading>
          {catalog && !editing && canWrite && !catalog.archived && (
            <Button
              size="sm"
              onClick={() => {
                setEditing(true);
                setSaved(false);
              }}
            >
              <Icon name="edit" />
              {t("catalogEdit")}
            </Button>
          )}
          {catalog?.archived && (
            <Badge tone="warning">{t("catalogArchived")}</Badge>
          )}
        </div>
        {catalog && !editing && (
          <div className="flex flex-wrap items-center gap-inline">
            {canWrite && (
              <ActionMenu
                label={t("catalogActions")}
                disabled={busy || dirty}
                items={[
                  {
                    id: "archive",
                    label: t(
                      catalog.archived ? "catalogRestore" : "catalogArchive",
                    ),
                    onAction: () =>
                      setAction(catalog.archived ? "restore" : "archive"),
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
          {error === "catalogConflict" && (
            <Button
              variant="secondary"
              className="mt-field"
              onClick={() => setAction("reload")}
            >
              {t("reloadCompany")}
            </Button>
          )}
          {!catalog && !creating && (
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
          {t("catalogSaved")}
        </Alert>
      )}
      {catalog && (
        <p className="flex items-center gap-inline text-sm text-muted">
          <Icon name="info" />
          {t("catalogSnapshotHint")}
        </p>
      )}
      {!editing && catalog && <CatalogOverview data={catalog.data} />}
      {!canWrite && catalog && (
        <p className="text-sm text-muted">{t("catalogReadOnly")}</p>
      )}
      {editing && (creating || catalog) && (
        <form
          className="space-y-field"
          noValidate
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <CatalogFields
            footer={
              canWrite &&
              !catalog?.archived && (
                <div className="profile-actions">
                  {creating ? (
                    <Link
                      className="ui-button ui-button-secondary ui-button-md"
                      to={`/companies/${companyId}/catalog`}
                    >
                      {t("cancelAction")}
                    </Link>
                  ) : (
                    <Button
                      variant="secondary"
                      disabled={busy}
                      onClick={() =>
                        dirty ? setAction("discard") : finishEditing()
                      }
                    >
                      {t("cancelAction")}
                    </Button>
                  )}
                  <Button type="submit" loading={busy} disabled={!dirty}>
                    {t(creating ? "catalogCreate" : "accountSave")}
                  </Button>
                </div>
              )
            }
            value={data}
            disabled={!canWrite || busy || !!catalog?.archived}
            fields={fields}
            onChange={(next) => {
              setFields((current) =>
                current.filter(
                  (key) =>
                    next[key as keyof CatalogData] ===
                    data[key as keyof CatalogData],
                ),
              );
              if (error === "catalogInvalid") setError("");
              setData(next);
              setSaved(false);
            }}
          />
          {!canWrite && (
            <p className="p-form-inset text-sm text-muted">
              {t("catalogReadOnly")}
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
            ? "catalogDiscard"
            : action === "reload"
              ? "reloadCompany"
              : action === "restore"
                ? "catalogRestore"
                : "catalogArchive",
        )}
        description={t(
          action === "discard"
            ? "catalogDiscardHint"
            : action === "reload"
              ? "catalogReloadHint"
              : "catalogArchiveHint",
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
                setData(catalog!.data);
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
                ? "catalogDiscard"
                : action === "reload"
                  ? "reloadCompany"
                  : action === "restore"
                    ? "catalogRestore"
                    : "catalogArchive",
            )}
          </Button>
        </div>
      </Dialog>
    </div>
  );
}
