import { useEffect, useState } from "react";
import { Link } from "react-router";
import {
  Alert,
  Button,
  Heading,
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  formatCalendarDate,
} from "@open-faktura/ui";
import {
  bankForeignReport,
  SalesFailure,
  type BankForeignReport,
} from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
import { moneyText } from "../../utils/invoice";
export function ExchangeDifferenceReport({
  companyId,
  from,
  to,
}: {
  companyId: string;
  from: string;
  to: string;
}) {
  const { t, language } = useAppTranslation(),
    { onExpired } = useWorkspace();
  const [page, setPage] = useState(1),
    [attempt, setAttempt] = useState(0),
    [value, setValue] = useState<BankForeignReport | null>(null),
    [error, setError] = useState(false);
  useEffect(() => {
    const c = new AbortController();
    setValue(null);
    setError(false);
    bankForeignReport(
      apiUrl,
      companyId,
      { from, to, page: String(page) },
      c.signal,
    )
      .then((v) => {
        if (!c.signal.aborted) setValue(v);
      })
      .catch((e) => {
        if (!c.signal.aborted) {
          setError(true);
          if (e instanceof SalesFailure && e.status === 401) onExpired();
        }
      });
    return () => c.abort();
  }, [companyId, from, to, page, attempt, onExpired]);
  return (
    <section
      aria-label={t("fxReportTitle")}
      className="workspace-panel space-y-field p-field"
    >
      <Heading level={2} size="section">
        {t("fxReportTitle")}
      </Heading>
      <p className="text-sm text-muted">{t("fxReportHelp")}</p>
      {error ? (
        <Alert tone="danger">
          {t("fxReportError")}{" "}
          <Button variant="secondary" onClick={() => setAttempt((x) => x + 1)}>
            {t("fxReportRetry")}
          </Button>
        </Alert>
      ) : !value ? (
        <p role="status">{t("bankForeignLoading")}</p>
      ) : (
        <>
          <dl className="flex flex-wrap gap-group">
            {(
              [
                ["gains", "fxReportGains"],
                ["losses", "fxReportLosses"],
                ["net", "fxReportNet"],
              ] as const
            ).map(([k, label]) => (
              <div key={k}>
                <dt className="text-sm text-muted">{t(label)}</dt>
                <dd className="font-semibold">
                  {moneyText(value[k], language)}
                </dd>
              </div>
            ))}
          </dl>
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  {(
                    [
                      "fxReportDate",
                      "fxReportMovement",
                      "fxReportDocument",
                      "fxReportPaid",
                      "fxReportBank",
                      "fxReportBook",
                      "fxReportDifference",
                    ] as const
                  ).map((k) => (
                    <TableHead key={k}>{t(k)}</TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <tbody>
                {value.items.map((row) => (
                  <TableRow key={row.id}>
                    <TableCell className="whitespace-nowrap">
                      {formatCalendarDate(row.booked_on, language)}
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <Link
                        className="underline"
                        to={`/companies/${companyId}/bank/${row.movement_id}`}
                      >
                        {row.movement_number}
                      </Link>
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <Link
                        className="underline"
                        to={`/companies/${companyId}/${row.target_kind === "invoice" ? "invoices" : "received-invoices"}/${row.document_id}`}
                      >
                        {row.document_number}
                      </Link>
                    </TableCell>
                    <TableCell className="text-right whitespace-nowrap">
                      {moneyText(
                        row.settlement.document_amount,
                        language,
                        row.settlement.currency,
                      )}
                    </TableCell>
                    <TableCell className="text-right whitespace-nowrap">
                      {moneyText(
                        row.bank_amount,
                        language,
                        row.settlement.bank_currency ?? "CZK",
                      )}
                    </TableCell>
                    <TableCell className="text-right whitespace-nowrap">
                      {moneyText(row.settlement.book_value_czk, language)}
                    </TableCell>
                    <TableCell className="text-right whitespace-nowrap">
                      {moneyText(row.settlement.difference_czk, language)}
                    </TableCell>
                  </TableRow>
                ))}
              </tbody>
            </Table>
          </div>
          {!value.total && <p>{t("fxReportEmpty")}</p>}
          <div className="flex justify-between gap-field">
            <span>{t("fxReportCount", { count: value.total })}</span>
            <div className="flex gap-control">
              <Button
                variant="secondary"
                disabled={page <= 1}
                onClick={() => setPage((p) => p - 1)}
              >
                {t("reportPrevious")}
              </Button>
              <Button
                variant="secondary"
                disabled={page * 25 >= value.total}
                onClick={() => setPage((p) => p + 1)}
              >
                {t("reportNext")}
              </Button>
            </div>
          </div>
        </>
      )}
    </section>
  );
}
