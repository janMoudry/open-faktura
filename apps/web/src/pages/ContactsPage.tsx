import { RegisterExport } from "../components/organisms/RegisterExport";
import { RegisterPagination } from "../components/molecules/RegisterPagination";
import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router";
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  Alert,
  Button,
  Input,
  FilterTabs,
  Heading,
  Icon,
} from "@open-faktura/ui";
import {
  ContactFailure,
  listContacts,
  type ContactPage,
} from "@open-faktura/api-client";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useAppTranslation, type AppKey } from "../locales";
import { contactError } from "../utils/contact";
import { apiUrl } from "../services/status";
export function ContactsPage() {
  const { companyId = "" } = useParams();
  const { company, onExpired } = useWorkspace();
  const { t } = useAppTranslation();
  const [params, setParams] = useSearchParams();
  const query = params.get("q") ?? "";
  const archived = params.get("archived") === "true";
  const page = Math.max(
    1,
    Math.min(10000, Math.trunc(Number(params.get("page"))) || 1),
  );
  const pageSize = [10, 25, 50, 100].includes(Number(params.get("size")))
    ? Number(params.get("size"))
    : 25;
  const change = (values: Record<string, string>) =>
    setParams({ ...Object.fromEntries(params), page: "1", ...values });
  const [input, setInput] = useState(query);
  const [result, setResult] = useState<ContactPage | null>(null);
  const [error, setError] = useState<AppKey | "">("");
  const [attempt, setAttempt] = useState(0);
  const key = JSON.stringify([
    companyId,
    query,
    archived,
    page,
    pageSize,
    attempt,
  ]);
  const [loaded, setLoaded] = useState("");
  useEffect(() => {
    setInput(query);
  }, [query, companyId]);
  useEffect(() => {
    const controller = new AbortController();
    setError("");
    listContacts(
      apiUrl,
      companyId,
      { query, archived, page, page_size: pageSize },
      controller.signal,
    )
      .then((value) => {
        if (!controller.signal.aborted) {
          setResult(value);
          setLoaded(key);
        }
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          if (error instanceof ContactFailure && error.status === 401)
            onExpired();
          else {
            setError(contactError(error));
            setLoaded(key);
          }
        }
      });
    return () => controller.abort();
  }, [key, companyId, query, archived, page, pageSize, onExpired]);
  const canWrite = company?.id === companyId && company.role !== "viewer";
  return (
    <div className="space-y-group">
      <div className="flex flex-wrap items-center justify-between gap-field">
        <div>
          <Heading level={1} size="page">
            {t("navContacts")}
          </Heading>
        </div>
        <RegisterExport
          key={companyId}
          company={companyId}
          onExpired={onExpired}
          selection={{
            kind: "contacts",
            search: { query, archived, page: 1, page_size: 100 },
          }}
        />
      </div>
      <div className="workspace-panel register-sheet">
        <div className="invoice-list-tabs">
          <FilterTabs
            label={t("contactStatus")}
            value={archived ? "archived" : "active"}
            options={[
              { value: "active", label: t("contactActive") },
              { value: "archived", label: t("contactArchived") },
            ]}
            onChange={(value) =>
              change({
                q: query,
                archived: String(value === "archived"),
                page: "1",
              })
            }
          />
        </div>
        <div className="p-field space-y-inline border-b border-border">
          <div className="flex flex-wrap items-center justify-between gap-inline">
            <form
              className="flex min-w-0 basis-full items-end gap-inline sm:basis-auto"
              onSubmit={(e) => {
                e.preventDefault();
                change({ q: input, archived: String(archived), page: "1" });
              }}
            >
              <Input
                className="w-full sm:w-72"
                aria-label={t("contactSearch")}
                placeholder={t("contactSearch")}
                value={input}
                maxLength={200}
                onChange={(e) => setInput(e.target.value)}
              />
              <Button
                variant="ghost"
                type="submit"
                aria-label={t("contactSearchAction")}
                title={t("contactSearchAction")}
              >
                <Icon name="search" />
              </Button>
            </form>
            {canWrite && (
              <Link
                className="ui-button ui-button-secondary ui-button-sm"
                to="new"
              >
                <Icon name="plus" />
                {t("contactNew")}
              </Link>
            )}
          </div>
          {query && (
            <div className="flex flex-wrap items-center gap-inline">
              <Button
                size="sm"
                variant="secondary"
                onClick={() =>
                  change({ q: "", archived: String(archived), page: "1" })
                }
              >
                <Icon name="close" />
                {t("draftSearchChip", { value: query })}
              </Button>
            </div>
          )}
        </div>
        {loaded !== key ? (
          <p role="status" className="p-form-inset">
            {t("contactsLoading")}
          </p>
        ) : error ? (
          <Alert tone="danger" role="alert">
            <p>{t(error)}</p>
            <Button
              variant="secondary"
              className="mt-field"
              onClick={() => setAttempt((v) => v + 1)}
            >
              {t("retry")}
            </Button>
          </Alert>
        ) : (
          result && (
            <>
              {result.items.length === 0 ? (
                <div className="p-section text-center">
                  <Heading level={2} size="card">
                    {t(
                      query
                        ? "contactNoResults"
                        : archived
                          ? "contactNoArchived"
                          : "contactEmpty",
                    )}
                  </Heading>
                  <p className="mt-inline text-sm text-muted">
                    {t(query ? "contactSearchHint" : "contactEmptyHint")}
                  </p>
                </div>
              ) : (
                <div
                  className="overflow-x-auto"
                  role="region"
                  tabIndex={0}
                  aria-label={t("navContacts")}
                >
                  <Table className="contact-register-table min-w-invoice-register table-fixed">
                    <TableHeader>
                      <TableRow>
                        <TableHead
                          scope="col"
                          className="w-1/4"
                          aria-sort="ascending"
                        >
                          {t("contactName")}
                        </TableHead>
                        <TableHead scope="col">
                          {t("registrationNumber")}
                        </TableHead>
                        <TableHead scope="col">{t("vatNumber")}</TableHead>
                        <TableHead scope="col">{t("contactPerson")}</TableHead>
                        <TableHead scope="col" className="w-1/5">
                          {t("email")}
                        </TableHead>
                        <TableHead scope="col">{t("phone")}</TableHead>
                        <TableHead scope="col" className="w-20 text-right">
                          {t("contactActions")}
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <tbody>
                      {result.items.map((contact) => (
                        <TableRow key={contact.id} className="hover:bg-canvas">
                          <TableCell className="font-medium">
                            <Link
                              className="block truncate hover:underline underline-offset-4"
                              title={contact.data.name}
                              to={contact.id}
                            >
                              {contact.data.name}
                            </Link>
                          </TableCell>
                          <TableCell className="whitespace-nowrap tabular-nums text-muted">
                            {contact.data.registration_number || "—"}
                          </TableCell>
                          <TableCell>
                            {contact.data.vat_number || "—"}
                          </TableCell>
                          <TableCell>
                            {contact.data.persons
                              .map((person) => person.name)
                              .join(", ") || "—"}
                          </TableCell>
                          <TableCell className="text-muted">
                            {contact.data.email || "—"}
                          </TableCell>
                          <TableCell>{contact.data.phone || "—"}</TableCell>
                          <TableCell className="text-right">
                            <Link
                              className="ui-button ui-button-ghost ui-button-sm"
                              to={
                                canWrite && !archived
                                  ? `${contact.id}?edit=true`
                                  : contact.id
                              }
                              aria-label={`${t(canWrite && !archived ? "contactEdit" : "contactDetail")}: ${contact.data.name}`}
                              title={t(
                                canWrite && !archived
                                  ? "contactEdit"
                                  : "contactDetail",
                              )}
                            >
                              <Icon
                                name={canWrite && !archived ? "edit" : "eye"}
                              />
                            </Link>
                          </TableCell>
                        </TableRow>
                      ))}
                    </tbody>
                  </Table>
                </div>
              )}
              <RegisterPagination
                page={page}
                pageSize={result.page_size}
                total={result.total}
                onPage={(page) =>
                  change({
                    q: query,
                    archived: String(archived),
                    page: String(page),
                  })
                }
                onPageSize={(size) => change({ size })}
              />
            </>
          )
        )}
      </div>
    </div>
  );
}
