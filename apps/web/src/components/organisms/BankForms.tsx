import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  DatePicker,
  Dialog,
  Input,
  Select,
  Table,
  TableHeader,
  TableHead,
  TableRow,
  TableCell,
} from "@open-faktura/ui";
import {
  bankPreview,
  bankStatementPreview,
  type BankStatement,
  type BankAccount,
  type BankPreview,
} from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
import { bankError } from "../../utils/bank";
import { useBankCommand } from "../../hooks/useBankCommand";
import { useWorkspace } from "../../hooks/useCompanyWorkspace";
import { apiUrl } from "../../services/status";
export type BankForm =
  | { kind: "account"; account?: BankAccount }
  | { kind: "import" }
  | { kind: "manual" };
export function BankForms({
  form,
  accounts,
  companyId,
  onClose,
  onSaved,
}: {
  form: BankForm;
  accounts: BankAccount[];
  companyId: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useAppTranslation(),
    { setPageDirty } = useWorkspace();
  const { execute, busy, error, setError } = useBankCommand(companyId);
  const existing = form.kind === "account" ? form.account : undefined;
  const [name, setName] = useState(existing?.data.name ?? ""),
    [iban, setIban] = useState(existing?.data.iban ?? ""),
    [archived, setArchived] = useState(existing?.archived ?? false);
  const [account, setAccount] = useState(
      accounts.find((a) => !a.archived)?.id ?? "",
    ),
    [csv, setCsv] = useState(""),
    [preview, setPreview] = useState<BankPreview | null>(null),
    [reading, setReading] = useState(false);
  const [format, setFormat] = useState<"csv" | BankStatement["format"]>("csv");
  const [filename, setFilename] = useState("");
  const [encoded, setEncoded] = useState("");
  const [fileVersion, setFileVersion] = useState(0);
  const statement = {
    account_id: account,
    format: format as BankStatement["format"],
    filename,
    content_base64: encoded,
  };
  const [amount, setAmount] = useState(""),
    [direction, setDirection] = useState("income"),
    [date, setDate] = useState(
      new Intl.DateTimeFormat("en-CA", {
        timeZone: "Europe/Prague",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
      }).format(new Date()),
    ),
    [vs, setVs] = useState(""),
    [partner, setPartner] = useState(""),
    [description, setDescription] = useState("");
  useEffect(() => {
    setPageDirty(true);
    return () => setPageDirty(false);
  }, [setPageDirty]);
  const title: AppKey =
    form.kind === "account"
      ? "bankAccount"
      : form.kind === "import"
        ? "bankImport"
        : "bankManual";
  const field = (
    label: AppKey,
    value: string,
    onChange: (v: string) => void,
    required = false,
  ) => (
    <label className="block space-y-inline">
      <span className="text-sm font-medium">{t(label)}</span>
      <Input
        required={required}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
  return (
    <Dialog
      open
      onClose={() => {
        if (!busy && !reading) onClose();
      }}
      title={t(title)}
      description={t(
        form.kind === "import"
          ? "bankImportHelp"
          : form.kind === "account"
            ? "bankAccountHelp"
            : "bankManualHelp",
      )}
    >
      <form
        className="space-y-field"
        onSubmit={async (e) => {
          e.preventDefault();
          const r = await execute(
            form.kind === "account"
              ? {
                  request_id: "",
                  action: "account",
                  id: existing?.id ?? null,
                  revision: existing?.revision ?? null,
                  data: { name, iban, currency: "CZK" },
                  archived,
                }
              : form.kind === "import"
                ? format === "csv"
                  ? {
                      request_id: "",
                      action: "import",
                      account_id: account,
                      csv,
                    }
                  : { request_id: "", action: "statement", ...statement }
                : {
                    request_id: "",
                    action: "manual",
                    account_id: account,
                    data: {
                      external_id: "",
                      booked_on: date,
                      amount:
                        (direction === "expense" ? "-" : "") +
                        amount.replace(",", "."),
                      currency: "CZK",
                      variable_symbol: vs,
                      counterparty: partner,
                      description,
                    },
                  },
          );
          if (r) onSaved();
        }}
      >
        {error && <Alert tone="danger">{t(error)}</Alert>}
        <fieldset disabled={busy || reading} className="space-y-field min-w-0">
          {form.kind === "account" ? (
            <>
              {field("bankName", name, setName, true)}
              {field("bankIban", iban, setIban, true)}
              <p className="text-sm text-muted">{t("bankCurrency")}</p>
              {existing && (
                <Select
                  label={t("bankAvailability")}
                  value={archived ? "archived" : "active"}
                  onChange={(v) => setArchived(v === "archived")}
                  options={[
                    { value: "active", label: t("bankActive") },
                    { value: "archived", label: t("bankArchived") },
                  ]}
                />
              )}
            </>
          ) : (
            <Select
              required
              label={t("bankAccount")}
              value={account}
              onChange={(v) => {
                setAccount(v);
                setPreview(null);
              }}
              options={accounts
                .filter((a) => !a.archived)
                .map((a) => ({ value: a.id, label: a.data.name }))}
            />
          )}
          {form.kind === "import" && (
            <>
              <Select
                label={t("bankFormat")}
                value={format}
                onChange={(v) => {
                  setFormat(v as typeof format);
                  setPreview(null);
                  setCsv("");
                  setEncoded("");
                  setFileVersion((v) => v + 1);
                  setError("");
                }}
                options={[
                  { value: "csv", label: t("bankFormatCsv") },
                  { value: "camt053", label: t("bankFormatCamt") },
                  { value: "fio_json", label: "Fio — JSON" },
                  { value: "moneta_xml", label: "MONETA — XML" },
                  { value: "rb_abo", label: t("bankFormatRb") },
                ]}
              />
              {format !== "csv" && (
                <>
                  <p className="text-sm text-muted">
                    {t(
                      format === "camt053"
                        ? "bankCamtHelp"
                        : format === "fio_json"
                          ? "bankFioHelp"
                          : format === "moneta_xml"
                            ? "bankMonetaHelp"
                            : "bankRbHelp",
                    )}
                  </p>
                  <Alert>{t("bankNativeHelp")}</Alert>
                </>
              )}
              {format === "csv" && (
                <Button
                  type="button"
                  variant="secondary"
                  onClick={() => {
                    const url = URL.createObjectURL(
                      new Blob(
                        [
                          "external_id,booked_on,amount,currency,variable_symbol,counterparty,description\n",
                        ],
                        { type: "text/csv;charset=utf-8" },
                      ),
                    );
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = "open-faktura-bank.csv";
                    a.click();
                    setTimeout(() => URL.revokeObjectURL(url), 1000);
                  }}
                >
                  {t("bankTemplate")}
                </Button>
              )}
              <label className="block space-y-inline">
                <span>{t("bankFile")}</span>
                <Input
                  type="file"
                  key={fileVersion}
                  accept={
                    format === "csv"
                      ? ".csv,text/csv"
                      : format === "fio_json"
                        ? ".json,application/json"
                        : format === "rb_abo"
                          ? ".gpc"
                          : ".xml,application/xml,text/xml"
                  }
                  onChange={async (e) => {
                    setPreview(null);
                    setCsv("");
                    setEncoded("");
                    setError("");
                    const f = e.target.files?.[0];
                    if (!f) return;
                    if (f.size > 1048576) {
                      setError("bankInvalid");
                      return;
                    }
                    setReading(true);
                    try {
                      setFilename(f.name);
                      if (format === "csv") setCsv(await f.text());
                      else {
                        const bytes = new Uint8Array(await f.arrayBuffer());
                        let binary = "";
                        for (const b of bytes) binary += String.fromCharCode(b);
                        setEncoded(btoa(binary));
                      }
                    } catch {
                      setError("bankFailed");
                    } finally {
                      setReading(false);
                    }
                  }}
                />
              </label>
              <Button
                type="button"
                variant="secondary"
                disabled={!(format === "csv" ? csv : encoded) || !account}
                onClick={async () => {
                  setReading(true);
                  setError("");
                  try {
                    setPreview(
                      format === "csv"
                        ? await bankPreview(apiUrl, companyId, account, csv)
                        : await bankStatementPreview(
                            apiUrl,
                            companyId,
                            statement,
                          ),
                    );
                  } catch (e) {
                    setError(bankError(e));
                  } finally {
                    setReading(false);
                  }
                }}
              >
                {t("bankPreview")}
              </Button>
              {preview && (
                <>
                  <p role="status">
                    {t("bankPreviewCount", {
                      count: preview.new_count,
                      duplicates: preview.duplicate_count,
                    })}
                  </p>
                  <div className="max-h-64 overflow-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>{t("bankExternalId")}</TableHead>
                          <TableHead>{t("bankAmount")}</TableHead>
                          <TableHead>{t("bankStatus")}</TableHead>
                        </TableRow>
                      </TableHeader>
                      <tbody>
                        {preview.rows.map((r) => (
                          <TableRow key={r.row}>
                            <TableCell>{r.data.external_id}</TableCell>
                            <TableCell>{r.data.amount}</TableCell>
                            <TableCell>
                              {t(
                                (
                                  {
                                    new: "bankNew",
                                    duplicate: "bankDuplicate",
                                    conflict: "bankRowConflict",
                                    invalid: "bankRowInvalid",
                                  } as const
                                )[r.status as "new"],
                              )}
                            </TableCell>
                          </TableRow>
                        ))}
                      </tbody>
                    </Table>
                  </div>
                  {!preview.valid && (
                    <Alert tone="danger">{t("bankImportInvalid")}</Alert>
                  )}
                </>
              )}
            </>
          )}
          {form.kind === "manual" && (
            <>
              <Select
                label={t("bankDirection")}
                value={direction}
                onChange={setDirection}
                options={[
                  { value: "income", label: t("bankIncome") },
                  { value: "expense", label: t("bankExpense") },
                ]}
              />
              <DatePicker
                required
                label={t("bankDate")}
                value={date}
                onChange={setDate}
              />
              {field("bankAmount", amount, setAmount, true)}
              {field("bankVs", vs, setVs)}
              {field("bankPartner", partner, setPartner)}
              {field("bankDescription", description, setDescription)}
            </>
          )}
        </fieldset>
        <div className="flex flex-wrap justify-end gap-control">
          <Button
            type="button"
            variant="secondary"
            disabled={busy || reading}
            onClick={onClose}
          >
            {t("bankClose")}
          </Button>
          <Button
            type="submit"
            disabled={
              busy ||
              reading ||
              (form.kind !== "account" && !account) ||
              (form.kind === "import" &&
                (!preview?.valid || preview.new_count === 0))
            }
          >
            {t(
              busy
                ? "bankSaving"
                : form.kind === "import"
                  ? "bankConfirmImport"
                  : "bankSave",
            )}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
