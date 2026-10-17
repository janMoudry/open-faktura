import { useEffect, useRef, useState } from "react";
import { Alert, Button, Checkbox, Dialog, Select } from "@open-faktura/ui";
import {
  exportRegister,
  SalesFailure,
  type ExportRequest,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
const invoiceColumns = [
  "number",
  "description",
  "status",
  "customer",
  "registration_number",
  "issued_on",
  "due_on",
  "currency",
  "net",
  "vat",
  "payable",
  "paid",
  "outstanding",
  "variable_symbol",
] as const;
const contactColumns = [
  "name",
  "registration_number",
  "vat_number",
  "email",
  "phone",
  "street",
  "city",
  "postal_code",
  "country",
  "bank_account",
  "iban",
  "archived",
] as const;
export function RegisterExport({
  company,
  selection,
  onExpired,
}: {
  company: string;
  selection: ExportRequest["selection"];
  onExpired: () => void;
}) {
  const { t, language } = useAppTranslation();
  const columns =
    selection.kind === "invoices" ? invoiceColumns : contactColumns;
  const [open, setOpen] = useState(false);
  const [format, setFormat] = useState<"csv" | "xlsx">("xlsx");
  const [selected, setSelected] = useState<string[]>([...columns]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppKey | "">("");
  const controller = useRef<AbortController | null>(null);
  useEffect(() => () => controller.current?.abort(), []);
  async function download() {
    setBusy(true);
    setError("");
    const c = new AbortController();
    controller.current = c;
    try {
      const blob = await exportRegister(
        apiUrl,
        company,
        {
          format,
          language,
          columns: selected as ExportRequest["columns"],
          selection,
        },
        c.signal,
      );
      if (c.signal.aborted) return;
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `open-faktura-${selection.kind}.${format}`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      setOpen(false);
    } catch (e) {
      if (!c.signal.aborted) {
        if (e instanceof SalesFailure && e.status === 401) onExpired();
        else
          setError(
            e instanceof SalesFailure && e.fields.includes("export_limit")
              ? "exportLimit"
              : "exportError",
          );
      }
    } finally {
      if (!c.signal.aborted) setBusy(false);
    }
  }
  return (
    <>
      <Button
        size="sm"
        variant="secondary"
        onClick={() => {
          setError("");
          setOpen(true);
        }}
      >
        {t("exportAction")}
      </Button>
      <Dialog
        open={open}
        onClose={() => {
          if (!busy) setOpen(false);
        }}
        title={t("exportTitle")}
        description={t("exportHint")}
      >
        <form
          className="space-y-field"
          onSubmit={(e) => {
            e.preventDefault();
            void download();
          }}
        >
          <Select
            label={t("exportFormat")}
            value={format}
            onChange={(v) => setFormat(v as "csv" | "xlsx")}
            disabled={busy}
            options={[
              { value: "xlsx", label: "Excel (.xlsx)" },
              { value: "csv", label: "CSV (.csv)" },
            ]}
          />
          <fieldset disabled={busy}>
            <legend className="font-medium">{t("exportColumns")}</legend>
            <div className="grid grid-cols-2 gap-x-field">
              {columns.map((c) => (
                <Checkbox
                  key={c}
                  label={t(`exportColumn_${c}` as AppKey)}
                  checked={selected.includes(c)}
                  onChange={(e) =>
                    setSelected(
                      e.target.checked
                        ? [...selected, c]
                        : selected.filter((x) => x !== c),
                    )
                  }
                />
              ))}
            </div>
          </fieldset>
          <p className="text-sm text-muted">{t("exportTextHint")}</p>
          {error && (
            <Alert tone="danger" role="alert">
              {t(error)}
            </Alert>
          )}
          <div className="flex justify-end gap-inline">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setOpen(false)}
            >
              {t("exportCancel")}
            </Button>
            <Button type="submit" disabled={busy || selected.length === 0}>
              {t(busy ? "exportBusy" : "exportDownload")}
            </Button>
          </div>
        </form>
      </Dialog>
    </>
  );
}
