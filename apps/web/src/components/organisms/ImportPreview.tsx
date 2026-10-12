import { moneyText } from "../../utils/invoice";
import { Link } from "react-router";
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
  Badge,
  Checkbox,
  Heading,
} from "@open-faktura/ui";
import type { ImportBatch } from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
export function ImportPreview({
  batch,
  company,
  skip,
  onSkip,
  disabled,
}: {
  batch: ImportBatch;
  company: string;
  skip: number[];
  onSkip: (v: number[]) => void;
  disabled: boolean;
}) {
  const { t, language } = useAppTranslation();
  const hasDetails = batch.rows.some((row) => row.fields.length > 0);
  const hasSelection = batch.rows.some((row) =>
    ["ready", "invalid", "conflict"].includes(row.status),
  );
  return (
    <section className="workspace-panel overflow-hidden">
      <div className="panel-toolbar">
        <Heading level={2} size="card">
          {t("importPreview")}
        </Heading>
        <p className="text-sm text-muted">
          {batch.source} · {batch.agenda} · {batch.rows.length}
        </p>
      </div>
      <div
        className="overflow-x-auto"
        tabIndex={0}
        role="region"
        aria-label={t("importPreview")}
      >
        <Table
          className={hasDetails || hasSelection ? "min-w-invoice-table" : ""}
        >
          <TableHeader>
            <TableRow>
              <TableHead>{t("importRecord")}</TableHead>
              <TableHead>{t("importStatus")}</TableHead>
              {hasDetails && <TableHead>{t("importDetails")}</TableHead>}
              {hasSelection && (
                <TableHead className="w-px">{t("importSkip")}</TableHead>
              )}
            </TableRow>
          </TableHeader>
          <tbody>
            {batch.rows.map((r) => (
              <TableRow key={r.index}>
                <TableCell className="max-w-sm">
                  <span className="font-semibold">
                    {r.target_id ? (
                      <Link
                        className="hover:underline underline-offset-4"
                        to={`/companies/${company}/${r.kind === "contact" ? "contacts" : "invoices"}/${r.target_id}`}
                      >
                        {r.label}
                      </Link>
                    ) : (
                      r.label
                    )}
                  </span>
                  <div className="mt-1 text-xs text-muted">{r.key}</div>
                </TableCell>
                <TableCell>
                  <Badge
                    tone={
                      ["invalid", "conflict"].includes(r.status)
                        ? "danger"
                        : r.status === "imported"
                          ? "success"
                          : "neutral"
                    }
                  >
                    {t(`importState_${r.status}` as AppKey)}
                  </Badge>
                </TableCell>
                {hasDetails && (
                  <TableCell className="max-w-sm text-sm text-muted">
                    {r.fields.map((f) => (
                      <p key={f}>
                        {t(
                          (
                            {
                              changed_source: "importChanged",
                              existing_target: "importCollision",
                              duplicate_key: "importDuplicateKey",
                              contact_key: "importMissingContact",
                              currency: "importCurrency",
                              payment_date: "importPaymentDate",
                            } as Record<string, AppKey>
                          )[f] ?? "importInvalidField",
                        )}
                        {![
                          "changed_source",
                          "existing_target",
                          "duplicate_key",
                          "contact_key",
                          "currency",
                          "payment_date",
                        ].includes(f) && ` (${f})`}
                      </p>
                    ))}
                  </TableCell>
                )}
                {hasSelection && (
                  <TableCell className="whitespace-nowrap">
                    {["ready", "invalid", "conflict"].includes(r.status) && (
                      <Checkbox
                        disabled={disabled}
                        label={t("importSkip")}
                        aria-label={t("importSkipRow", { key: r.key })}
                        checked={skip.includes(r.index)}
                        onChange={(e) =>
                          onSkip(
                            e.target.checked
                              ? [...skip, r.index]
                              : skip.filter((v) => v !== r.index),
                          )
                        }
                      />
                    )}
                  </TableCell>
                )}
              </TableRow>
            ))}
          </tbody>
        </Table>
      </div>
      {batch.totals.length > 0 && (
        <div className="border-t border-border p-field space-y-field">
          <Heading level={3} size="label">
            {t("importReconcile")}
          </Heading>
          {batch.totals.map((v) => (
            <dl key={v.currency} className="grid gap-field sm:grid-cols-4">
              {(["source", "imported", "skipped", "payments"] as const).map(
                (k) => (
                  <div key={k}>
                    <dt className="text-sm text-muted">
                      {t(`importTotal_${k}` as AppKey)}
                    </dt>
                    <dd className="tabular-nums font-semibold">
                      {moneyText(v[k], language, v.currency)}
                    </dd>
                  </div>
                ),
              )}
            </dl>
          ))}
        </div>
      )}
    </section>
  );
}
