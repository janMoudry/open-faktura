import { useState } from "react";
import { Link, useParams } from "react-router";
import {
  Alert,
  Button,
  Heading,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import { useAppTranslation } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { useSalesTrash, useSalesTrashCommand } from "../hooks/useSalesTrash";
export function SalesTrashPage() {
  const { companyId = "" } = useParams();
  const { company } = useWorkspace();
  const { t, language } = useAppTranslation();
  const [page, setPage] = useState(1);
  const { data, error, loading, reload } = useSalesTrash(companyId, page);
  const action = useSalesTrashCommand();
  const editable = company?.id === companyId && company.role !== "viewer";
  return (
    <div className="space-y-group">
      <Link
        className="text-sm text-muted hover:underline"
        to={`/companies/${companyId}/quotes`}
      >
        {t("salesQuotes")}
      </Link>
      <header className="space-y-control">
        <Heading level={1} size="page">
          {t("salesTrashTitle")}
        </Heading>
        <p className="text-sm text-muted">{t("salesTrashHelp")}</p>
      </header>
      {error || action.error ? (
        <Alert tone="danger" role="alert">
          {t("salesTrashError")}
          <Button variant="secondary" onClick={reload}>
            {t("retry")}
          </Button>
        </Alert>
      ) : null}
      {loading ? (
        <p role="status">{t("draftLoading")}</p>
      ) : (
        data && (
          <>
            <div className="overflow-x-auto">
              <Table aria-label={t("salesTrashTitle")}>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("salesTrashName")}</TableHead>
                    <TableHead>{t("navContacts")}</TableHead>
                    <TableHead>{t("salesTrashDate")}</TableHead>
                    <TableHead>
                      <span className="sr-only">{t("salesTrashRestore")}</span>
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <tbody>
                  {data.items.map((item) => (
                    <TableRow key={item.id}>
                      <TableCell>
                        <span className="block">
                          {item.title || t("salesTrashUntitled")}
                        </span>
                        <span className="text-xs text-muted">
                          {t(
                            item.kind === "quote"
                              ? "salesQuotes"
                              : "salesTemplates",
                          )}
                        </span>
                      </TableCell>
                      <TableCell>{item.customer || "—"}</TableCell>
                      <TableCell>
                        {item.deleted_at
                          ? new Intl.DateTimeFormat(language, {
                              dateStyle: "medium",
                              timeStyle: "short",
                              timeZone:
                                company?.settings.timezone ?? "Europe/Prague",
                            }).format(new Date(item.deleted_at))
                          : "—"}
                      </TableCell>
                      <TableCell>
                        {editable && (
                          <Button
                            variant="secondary"
                            disabled={action.busy}
                            onClick={async () => {
                              if (
                                await action.execute(companyId, {
                                  request_id: crypto.randomUUID(),
                                  id: item.id,
                                  revision: item.revision,
                                  action: "restore",
                                })
                              )
                                reload();
                            }}
                          >
                            {t("salesTrashRestore")}
                          </Button>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </tbody>
              </Table>
            </div>
            {data.items.length === 0 && (
              <p className="text-sm text-muted">{t("salesTrashEmpty")}</p>
            )}
            <div className="flex items-center justify-between gap-field">
              <span className="text-sm text-muted">
                {t("salesTrashCount", { count: data.total })}
              </span>
              <div className="flex gap-control">
                <Button
                  variant="secondary"
                  disabled={page === 1}
                  onClick={() => setPage((v) => v - 1)}
                >
                  {t("contactPrevious")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={page * 25 >= data.total}
                  onClick={() => setPage((v) => v + 1)}
                >
                  {t("contactNext")}
                </Button>
              </div>
            </div>
          </>
        )
      )}
    </div>
  );
}
